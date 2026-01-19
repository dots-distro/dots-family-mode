#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_probe_read_user_str,
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub comm: [u8; 16],
    pub cmdline: [u8; 512], // Command line arguments
    pub event_type: u32,    // 0 = exec, 1 = exit
}

// Kernel task structure for accessing task fields
#[repr(C)]
struct task_struct {
    _ptr: *mut core::ffi::c_void,
    real_parent: task_parent,
    argv: [*mut core::ffi::c_char; 64],
    envp: [*mut core::ffi::c_char; 64],
}

#[repr(C)]
struct task_parent {
    pid: u32,
}

#[map]
static PROCESS_EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(ctx) {
        Ok(ret) => ret,
        Err(_) => 1,
    }
}

fn try_sched_process_exec(ctx: TracePointContext) -> Result<u32, u32> {
    let task = unsafe { ctx.task as *const task_struct };
    let mut cmdline = [0u8; 512];

    unsafe {
        let mut i = 0;
        while i < 511 && i < task.argv.len() {
            let arg_ptr = *task.argv.add(i);
            if !arg_ptr.is_null() {
                let arg_len = bpf_probe_read_user_str(arg_ptr, &mut cmdline[i as usize]);
                if arg_len > 0 && i + arg_len < 511 {
                    i += arg_len + 1; // +1 for null terminator
                }
            }
            i += 1;
        }
    }

    let uid_gid = unsafe { bpf_get_current_uid_gid() };
    let event = ProcessEvent {
        pid: unsafe { bpf_get_current_pid_tgid() as u32 },
        ppid: unsafe { task.real_parent.pid },
        uid: uid_gid as u32,
        gid: (uid_gid >> 32) as u32,
        comm: [0; 16],
        cmdline,
        event_type: 0, // exec
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    Ok(0)
}

#[tracepoint]
pub fn sched_process_exit(_ctx: TracePointContext) -> u32 {
    let event = ProcessEvent {
        pid: 0,  // Simplified for compatibility
        ppid: 0, // Simplified for compatibility
        uid: 0,  // Simplified for compatibility
        gid: 0,  // Simplified for compatibility
        comm: [0; 16],
        event_type: 1, // exit event
    };

    if let Some(mut buf) = PROCESS_EVENTS.reserve::<ProcessEvent>(0) {
        buf.write(event);
        buf.submit(0);
    }

    0
}
