#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_probe_read_kernel},
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
    EbpfContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkEvent {
    pub event_type: u32, // 1 = connect, 2 = bind, 3 = accept, 4 = close
    pub pid: u32,
    pub comm: [u8; 16], // Process name
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,    // TCP = 6, UDP = 17
    pub bytes_sent: u64, // For data transfer events
    pub bytes_received: u64,
}

#[map]
static NETWORK_EVENTS: RingBuf = RingBuf::with_byte_size(512 * 1024, 0);

#[kprobe]
pub fn tcp_connect(ctx: ProbeContext) -> u32 {
    // Get PID from current process
    // bpf_get_current_pid_tgid() returns u64 where high 32 bits = tgid, low 32 bits = pid
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32; // TGID (actual process ID)

    // Get process name
    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // Phase 2: Extract socket information from struct sock *
    // The first argument to tcp_connect is struct sock *sk
    let mut src_addr: u32 = 0;
    let mut dst_addr: u32 = 0;
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;

    // Try to get the socket pointer from the first argument
    if let Some(sk_ptr) = ctx.arg::<u64>(0) {
        if sk_ptr != 0 {
            // struct sock has a __sk_common field at the beginning
            // struct sock_common contains network addressing fields
            // Offsets are approximate and may need adjustment for different kernels
            // This is a best-effort approach without BTF/CO-RE

            // Try to read source address (skc_rcv_saddr offset ~24 bytes in sock_common)
            let src_addr_ptr = (sk_ptr + 24) as *const u32;
            src_addr = unsafe { bpf_probe_read_kernel(src_addr_ptr) }.unwrap_or(0);

            // Try to read destination address (skc_daddr offset ~28 bytes)
            let dst_addr_ptr = (sk_ptr + 28) as *const u32;
            dst_addr = unsafe { bpf_probe_read_kernel(dst_addr_ptr) }.unwrap_or(0);

            // Try to read source port (skc_num offset ~32 bytes)
            let src_port_ptr = (sk_ptr + 32) as *const u16;
            src_port = unsafe { bpf_probe_read_kernel(src_port_ptr) }.unwrap_or(0);

            // Try to read destination port (skc_dport offset ~34 bytes)
            let dst_port_ptr = (sk_ptr + 34) as *const u16;
            dst_port = unsafe { bpf_probe_read_kernel(dst_port_ptr) }.unwrap_or(0);

            // Convert from network byte order (big endian) to host byte order
            dst_port = u16::from_be(dst_port);
        }
    }

    let event = NetworkEvent {
        event_type: 1, // TCP connect
        pid,
        comm,
        src_addr,
        dst_addr,
        src_port,
        dst_port,
        protocol: 6, // TCP
        bytes_sent: 0,
        bytes_received: 0,
    };

    if let Some(mut buf) = NETWORK_EVENTS.reserve::<NetworkEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

// Phase 3: Track TCP send bandwidth
#[kprobe]
pub fn tcp_sendmsg(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // tcp_sendmsg signature: int tcp_sendmsg(struct sock *sk, struct msghdr *msg, size_t size)
    // Third argument is the size being sent
    let bytes_sent = ctx.arg::<u64>(2).unwrap_or(0);

    // Try to get socket info from first argument
    let mut src_addr: u32 = 0;
    let mut dst_addr: u32 = 0;
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;

    if let Some(sk_ptr) = ctx.arg::<u64>(0) {
        if sk_ptr != 0 {
            let src_addr_ptr = (sk_ptr + 24) as *const u32;
            src_addr = unsafe { bpf_probe_read_kernel(src_addr_ptr) }.unwrap_or(0);

            let dst_addr_ptr = (sk_ptr + 28) as *const u32;
            dst_addr = unsafe { bpf_probe_read_kernel(dst_addr_ptr) }.unwrap_or(0);

            let src_port_ptr = (sk_ptr + 32) as *const u16;
            src_port = unsafe { bpf_probe_read_kernel(src_port_ptr) }.unwrap_or(0);

            let dst_port_ptr = (sk_ptr + 34) as *const u16;
            dst_port = unsafe { bpf_probe_read_kernel(dst_port_ptr) }.unwrap_or(0);

            dst_port = u16::from_be(dst_port);
        }
    }

    let event = NetworkEvent {
        event_type: 2, // TCP send
        pid,
        comm,
        src_addr,
        dst_addr,
        src_port,
        dst_port,
        protocol: 6, // TCP
        bytes_sent,
        bytes_received: 0,
    };

    if let Some(mut buf) = NETWORK_EVENTS.reserve::<NetworkEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

// Phase 3: Track TCP receive bandwidth
#[kprobe]
pub fn tcp_recvmsg(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let comm = bpf_get_current_comm().unwrap_or([0u8; 16]);

    // tcp_recvmsg signature: int tcp_recvmsg(struct sock *sk, struct msghdr *msg, size_t len, ...)
    // Third argument is the requested size
    // Note: The actual received size is the return value, but we track the requested size here
    let bytes_received = ctx.arg::<u64>(2).unwrap_or(0);

    // Try to get socket info from first argument
    let mut src_addr: u32 = 0;
    let mut dst_addr: u32 = 0;
    let mut src_port: u16 = 0;
    let mut dst_port: u16 = 0;

    if let Some(sk_ptr) = ctx.arg::<u64>(0) {
        if sk_ptr != 0 {
            let src_addr_ptr = (sk_ptr + 24) as *const u32;
            src_addr = unsafe { bpf_probe_read_kernel(src_addr_ptr) }.unwrap_or(0);

            let dst_addr_ptr = (sk_ptr + 28) as *const u32;
            dst_addr = unsafe { bpf_probe_read_kernel(dst_addr_ptr) }.unwrap_or(0);

            let src_port_ptr = (sk_ptr + 32) as *const u16;
            src_port = unsafe { bpf_probe_read_kernel(src_port_ptr) }.unwrap_or(0);

            let dst_port_ptr = (sk_ptr + 34) as *const u16;
            dst_port = unsafe { bpf_probe_read_kernel(dst_port_ptr) }.unwrap_or(0);

            dst_port = u16::from_be(dst_port);
        }
    }

    let event = NetworkEvent {
        event_type: 3, // TCP receive
        pid,
        comm,
        src_addr,
        dst_addr,
        src_port,
        dst_port,
        protocol: 6, // TCP
        bytes_sent: 0,
        bytes_received,
    };

    if let Some(mut buf) = NETWORK_EVENTS.reserve::<NetworkEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
