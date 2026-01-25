#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_comm, bpf_get_current_pid_tgid},
    macros::{kprobe, map},
    maps::RingBuf,
    programs::ProbeContext,
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
pub fn tcp_connect(_ctx: ProbeContext) -> u32 {
    // Get PID from current process
    // bpf_get_current_pid_tgid() returns u64 where high 32 bits = tgid, low 32 bits = pid
    let pid_tgid = unsafe { bpf_get_current_pid_tgid() };
    let pid = (pid_tgid >> 32) as u32; // TGID (actual process ID)

    // Get process name
    let comm = unsafe { bpf_get_current_comm() }.unwrap_or([0u8; 16]);

    // Note: Socket address/port extraction requires reading from socket struct
    // This needs kernel structure definitions and BTF/CO-RE support
    // For Phase 1, we capture PID and process name which is already valuable
    // TODO Phase 2: Extract socket info from sk_common or inet_sock structs

    let event = NetworkEvent {
        event_type: 1, // TCP connect
        pid,
        comm,
        src_addr: 0, // TODO: Extract from socket->sk_common.skc_rcv_saddr
        dst_addr: 0, // TODO: Extract from socket->sk_common.skc_daddr
        src_port: 0, // TODO: Extract from socket->sk_common.skc_num
        dst_port: 0, // TODO: Extract from socket->sk_common.skc_dport
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

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
