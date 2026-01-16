#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::xdp_action,
    helpers::bpf_get_current_pid_tgid,
    macros::{kprobe, map, xdp},
    maps::{HashMap, PerfEventArray},
    programs::{ProbeContext, XdpContext},
    EbpfContext,
};
use aya_log_ebpf::info;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

// Event types
const EVENT_CONNECT: u32 = 1;
const EVENT_BIND: u32 = 2;
const EVENT_PACKET: u32 = 3;
const EVENT_STATE_CHANGE: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NetworkEvent {
    pub event_type: u32,
    pub pid: u32,
    pub src_addr: u32,
    pub dst_addr: u32,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: u8,
    pub padding: u8,
}

// Maps for event communication
#[map(name = "NET_EVENTS")]
static mut NET_EVENTS: PerfEventArray<NetworkEvent> = PerfEventArray::with_max_entries(1024, 0);

#[map(name = "NET_STATS")]
static mut NET_STATS: HashMap<u32, u64> = HashMap::with_max_entries(16, 0);

#[map(name = "ACTIVE_CONNECTIONS")]
static mut ACTIVE_CONNECTIONS: HashMap<u64, NetworkEvent> = HashMap::with_max_entries(1024, 0);

#[map(name = "NET_CONFIG")]
static mut NET_CONFIG: HashMap<u32, u16> = HashMap::with_max_entries(16, 0);

#[kprobe(name = "trace_inet_sock_connect")]
pub fn trace_inet_sock_connect(ctx: ProbeContext) -> u32 {
    match try_trace_connect(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_trace_connect(ctx: ProbeContext) -> Result<u32, u32> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    // Extract socket address information from function parameters
    // This is simplified - real implementation would parse sockaddr structure
    let event = NetworkEvent {
        event_type: EVENT_CONNECT,
        pid,
        src_addr: 0, // Would extract from socket
        dst_addr: 0, // Would extract from connect parameters
        src_port: 0,
        dst_port: 0,
        protocol: 6, // TCP
        padding: 0,
    };

    unsafe {
        NET_EVENTS.output(&ctx, &event, 0);
    }
    increment_net_stat(1); // connect counter

    info!(&ctx, "Socket connect: pid={}", pid);

    Ok(0)
}

#[kprobe(name = "trace_inet_bind")]
pub fn trace_inet_bind(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let event = NetworkEvent {
        event_type: EVENT_BIND,
        pid,
        src_addr: 0, // Would extract from socket
        dst_addr: 0,
        src_port: 0, // Would extract from bind parameters
        dst_port: 0,
        protocol: 6, // TCP
        padding: 0,
    };

    unsafe {
        NET_EVENTS.output(&ctx, &event, 0);
    }
    increment_net_stat(2); // bind counter

    info!(&ctx, "Socket bind: pid={}", pid);

    0
}

#[kprobe(name = "trace_tcp_set_state")]
pub fn trace_tcp_set_state(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;

    let event = NetworkEvent {
        event_type: EVENT_STATE_CHANGE,
        pid,
        src_addr: 0,
        dst_addr: 0,
        src_port: 0,
        dst_port: 0,
        protocol: 6, // TCP
        padding: 0,
    };

    unsafe {
        NET_EVENTS.output(&ctx, &event, 0);
    }
    increment_net_stat(4); // state change counter

    info!(&ctx, "TCP state change: pid={}", pid);

    0
}

#[xdp(name = "xdp_packet_capture")]
pub fn xdp_packet_capture(ctx: XdpContext) -> u32 {
    match try_packet_capture(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

fn try_packet_capture(ctx: XdpContext) -> Result<u32, ()> {
    let ethhdr: *const EthHdr =
        unsafe { aya_ebpf::helpers::bpf_xdp_load_bytes(&ctx, 0, core::mem::size_of::<EthHdr>()) };

    if ethhdr.is_null() {
        return Ok(xdp_action::XDP_PASS);
    }

    match unsafe { (*ethhdr).ether_type } {
        EtherType::Ipv4 => {
            // Parse IPv4 header
            let ipv4hdr: *const Ipv4Hdr = unsafe {
                aya_ebpf::helpers::bpf_xdp_load_bytes(
                    &ctx,
                    EthHdr::LEN,
                    core::mem::size_of::<Ipv4Hdr>(),
                )
            };

            if ipv4hdr.is_null() {
                return Ok(xdp_action::XDP_PASS);
            }

            let protocol = unsafe { (*ipv4hdr).proto };
            let src_addr = unsafe { u32::from_be((*ipv4hdr).src_addr) };
            let dst_addr = unsafe { u32::from_be((*ipv4hdr).dst_addr) };

            // Check if we should monitor this port
            let (src_port, dst_port) = match protocol {
                IpProto::Tcp => {
                    let tcphdr: *const TcpHdr = unsafe {
                        aya_ebpf::helpers::bpf_xdp_load_bytes(
                            &ctx,
                            EthHdr::LEN + Ipv4Hdr::LEN,
                            core::mem::size_of::<TcpHdr>(),
                        )
                    };

                    if tcphdr.is_null() {
                        return Ok(xdp_action::XDP_PASS);
                    }

                    let src = unsafe { u16::from_be((*tcphdr).source) };
                    let dst = unsafe { u16::from_be((*tcphdr).dest) };
                    (src, dst)
                }
                IpProto::Udp => {
                    let udphdr: *const UdpHdr = unsafe {
                        aya_ebpf::helpers::bpf_xdp_load_bytes(
                            &ctx,
                            EthHdr::LEN + Ipv4Hdr::LEN,
                            core::mem::size_of::<UdpHdr>(),
                        )
                    };

                    if udphdr.is_null() {
                        return Ok(xdp_action::XDP_PASS);
                    }

                    let src = unsafe { u16::from_be((*udphdr).source) };
                    let dst = unsafe { u16::from_be((*udphdr).dest) };
                    (src, dst)
                }
                _ => (0, 0),
            };

            if should_capture_port(dst_port) || should_capture_port(src_port) {
                let event = NetworkEvent {
                    event_type: EVENT_PACKET,
                    pid: 0, // Not available in XDP context
                    src_addr,
                    dst_addr,
                    src_port,
                    dst_port,
                    protocol: protocol as u8,
                    padding: 0,
                };

                unsafe {
                    NET_EVENTS.output(&ctx, &event, 0);
                }
                increment_net_stat(3); // packet counter
            }
        }
        _ => {}
    }

    Ok(xdp_action::XDP_PASS)
}

fn should_capture_port(port: u16) -> bool {
    // Check configuration for monitored ports
    if let Some(monitor_port) = unsafe { NET_CONFIG.get(&0) } {
        return *monitor_port == 0 || *monitor_port == port;
    }

    // Default: capture common service ports
    match port {
        22 | 80 | 443 | 8080 | 8443 => true,
        _ => false,
    }
}

fn increment_net_stat(stat_type: u32) {
    if let Some(count) = unsafe { NET_STATS.get(&stat_type) } {
        let new_count = *count + 1;
        let _ = unsafe { NET_STATS.insert(&stat_type, &new_count, 0) };
    } else {
        let _ = unsafe { NET_STATS.insert(&stat_type, &1u64, 0) };
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
