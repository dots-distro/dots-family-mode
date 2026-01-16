#![no_std]
#![no_main]

use aya_ebpf::{
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
    BpfContext,
};
use aya_log_ebpf::info;

// Process event structure that matches userspace
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub filename: [u8; 256],
    pub event_type: u32, // 0 = exec, 1 = exit
    pub timestamp: u64,
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sched_process_exec(ctx: TracePointContext) -> Result<u32, u32> {
    let pid = ctx.pid();
    let uid = ctx.uid();
    let gid = ctx.gid();

    // Get parent PID from task struct
    let ppid = unsafe {
        let task =
            aya_ebpf::helpers::bpf_get_current_task() as *const aya_ebpf::bindings::task_struct;
        if task.is_null() {
            return Ok(0);
        }
        let parent = (*task).parent;
        if parent.is_null() {
            return Ok(0);
        }
        (*parent).pid
    };

    let mut event = ProcessEvent {
        pid,
        ppid,
        uid,
        gid,
        comm: [0; 16],
        filename: [0; 256],
        event_type: 0, // exec event
        timestamp: unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() },
    };

    // Get process name
    let comm_result = unsafe {
        aya_ebpf::helpers::bpf_get_current_comm(&mut event.comm as *mut [u8; 16] as *mut u8, 16)
    };

    if comm_result != 0 {
        info!(&ctx, "Failed to get comm for PID {}", pid);
    }

    // Try to get filename from tracepoint args
    if let Ok(filename_ptr) = ctx.read_at::<u64>(16) {
        if filename_ptr != 0 {
            let ret = unsafe {
                aya_ebpf::helpers::bpf_probe_read_user_str(
                    &mut event.filename as *mut [u8; 256] as *mut u8,
                    256,
                    filename_ptr as *const u8,
                )
            };
            if ret < 0 {
                info!(&ctx, "Failed to read filename for PID {}", pid);
            }
        }
    }

    // Submit event to ring buffer
    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    info!(
        &ctx,
        "Process exec: PID {} PPID {} CMD {:?}",
        pid,
        ppid,
        unsafe { core::str::from_utf8_unchecked(&event.comm[..8]) }
    );

    Ok(0)
}

#[tracepoint]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    match try_sched_process_exit(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sched_process_exit(ctx: TracePointContext) -> Result<u32, u32> {
    let pid = ctx.pid();
    let uid = ctx.uid();
    let gid = ctx.gid();

    let mut event = ProcessEvent {
        pid,
        ppid: 0, // Not relevant for exit
        uid,
        gid,
        comm: [0; 16],
        filename: [0; 256],
        event_type: 1, // exit event
        timestamp: unsafe { aya_ebpf::helpers::bpf_ktime_get_ns() },
    };

    // Get process name
    let comm_result = unsafe {
        aya_ebpf::helpers::bpf_get_current_comm(&mut event.comm as *mut [u8; 16] as *mut u8, 16)
    };

    if comm_result != 0 {
        info!(&ctx, "Failed to get comm for exiting PID {}", pid);
    }

    // Submit event to ring buffer
    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    info!(&ctx, "Process exit: PID {} CMD {:?}", pid, unsafe {
        core::str::from_utf8_unchecked(&event.comm[..8])
    });

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
