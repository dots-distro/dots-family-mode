#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{bpf_get_current_pid_tgid, bpf_probe_read_kernel, bpf_probe_read_user_str},
    macros::{kprobe, map},
    maps::{HashMap, PerfEventArray},
    programs::ProbeContext,
    BpfContext,
};
use aya_log_ebpf::info;

// Event types
const EVENT_OPEN: u32 = 1;
const EVENT_READ: u32 = 2;
const EVENT_WRITE: u32 = 3;
const EVENT_DELETE: u32 = 4;
const EVENT_CHMOD: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FilesystemEvent {
    pub event_type: u32,
    pub pid: u32,
    pub tgid: u32,
    pub fd: u32,
    pub filename: [u8; 256],
}

// Maps for event communication
#[map(name = "FS_EVENTS")]
static mut FS_EVENTS: PerfEventArray<FilesystemEvent> = PerfEventArray::new(0);

#[map(name = "FS_STATS")]
static mut FS_STATS: HashMap<u32, u64> = HashMap::new(0);

#[map(name = "RECENT_FILES")]
static mut RECENT_FILES: HashMap<u64, FilesystemEvent> = HashMap::new(0);

#[map(name = "PATH_FILTER")]
static mut PATH_FILTER: HashMap<u32, [u8; 256]> = HashMap::new(0);

#[kprobe(name = "trace_do_sys_open")]
pub fn trace_do_sys_open(ctx: ProbeContext) -> u32 {
    match try_trace_open(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_trace_open(ctx: ProbeContext) -> Result<u32, u32> {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    let mut event = FilesystemEvent {
        event_type: EVENT_OPEN,
        pid,
        tgid,
        fd: 0,
        filename: [0; 256],
    };

    // Get filename from system call parameters
    // Parameters: int dfd, const char __user *filename, int flags, umode_t mode
    let filename_ptr = ctx.arg(1).ok_or(1u32)?;

    let ret = unsafe {
        bpf_probe_read_user_str(event.filename.as_mut_ptr(), 256, filename_ptr as *const u8)
    };

    if ret < 0 {
        return Err(1);
    }

    // Check if we should filter this path
    if should_filter_path(&event.filename) {
        return Ok(0);
    }

    FS_EVENTS.output(&ctx, &event, 0);
    increment_fs_stat(1); // open counter

    // Store in recent files (using timestamp as key)
    let timestamp = unsafe { aya_bpf::helpers::bpf_ktime_get_ns() };
    let _ = unsafe { RECENT_FILES.insert(&timestamp, &event, 0) };

    info!(
        &ctx,
        "File open: pid={} file={:?}",
        pid,
        &event.filename[..32]
    );

    Ok(0)
}

#[kprobe(name = "trace_vfs_read")]
pub fn trace_vfs_read(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    // Parameters: struct file *file, char __user *buf, size_t count, loff_t *pos
    let file_ptr = ctx.arg(0).ok_or(1u32).unwrap_or(0);

    let mut event = FilesystemEvent {
        event_type: EVENT_READ,
        pid,
        tgid,
        fd: 0,
        filename: [0; 256],
    };

    // Extract filename from file structure (simplified)
    // In a real implementation, this would properly parse the file structure
    let _ = extract_filename_from_file(file_ptr, &mut event.filename);

    if should_filter_path(&event.filename) {
        return 0;
    }

    FS_EVENTS.output(&ctx, &event, 0);
    increment_fs_stat(2); // read counter

    info!(&ctx, "File read: pid={}", pid);

    0
}

#[kprobe(name = "trace_vfs_write")]
pub fn trace_vfs_write(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    let file_ptr = ctx.arg(0).ok_or(1u32).unwrap_or(0);

    let mut event = FilesystemEvent {
        event_type: EVENT_WRITE,
        pid,
        tgid,
        fd: 0,
        filename: [0; 256],
    };

    let _ = extract_filename_from_file(file_ptr, &mut event.filename);

    if should_filter_path(&event.filename) {
        return 0;
    }

    FS_EVENTS.output(&ctx, &event, 0);
    increment_fs_stat(3); // write counter

    info!(&ctx, "File write: pid={}", pid);

    0
}

#[kprobe(name = "trace_vfs_unlink")]
pub fn trace_vfs_unlink(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    let mut event = FilesystemEvent {
        event_type: EVENT_DELETE,
        pid,
        tgid,
        fd: 0,
        filename: [0; 256],
    };

    // Extract filename from unlink parameters
    let dentry_ptr = ctx.arg(1).ok_or(1u32).unwrap_or(0);
    let _ = extract_filename_from_dentry(dentry_ptr, &mut event.filename);

    FS_EVENTS.output(&ctx, &event, 0);
    increment_fs_stat(4); // delete counter

    info!(&ctx, "File delete: pid={}", pid);

    0
}

#[kprobe(name = "trace_notify_change")]
pub fn trace_notify_change(ctx: ProbeContext) -> u32 {
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32;
    let tgid = pid_tgid as u32;

    let mut event = FilesystemEvent {
        event_type: EVENT_CHMOD,
        pid,
        tgid,
        fd: 0,
        filename: [0; 256],
    };

    // Extract filename from inode/dentry
    let dentry_ptr = ctx.arg(0).ok_or(1u32).unwrap_or(0);
    let _ = extract_filename_from_dentry(dentry_ptr, &mut event.filename);

    FS_EVENTS.output(&ctx, &event, 0);
    increment_fs_stat(5); // chmod counter

    info!(&ctx, "File permission change: pid={}", pid);

    0
}

// Helper functions
fn should_filter_path(path: &[u8; 256]) -> bool {
    // Check if path matches any filter patterns
    for i in 0..10 {
        if let Some(filter_path) = unsafe { PATH_FILTER.get(&i) } {
            if paths_match(path, filter_path) {
                return false; // Path matches filter, don't filter out
            }
        }
    }

    // Default filtering: skip common system paths that generate too much noise
    let path_str = unsafe { core::str::from_utf8_unchecked(&path[..64]) };

    match path_str {
        s if s.starts_with("/proc/") => true,
        s if s.starts_with("/sys/") => true,
        s if s.starts_with("/dev/") => true,
        s if s.starts_with("/tmp/.") => true,
        _ => false,
    }
}

fn paths_match(path1: &[u8; 256], path2: &[u8; 256]) -> bool {
    // Simple prefix matching
    for i in 0..256 {
        if path1[i] == 0 && path2[i] == 0 {
            return true;
        }
        if path1[i] != path2[i] {
            return false;
        }
    }
    true
}

fn extract_filename_from_file(file_ptr: u64, filename: &mut [u8; 256]) -> i32 {
    // This is a simplified implementation
    // Real implementation would parse the file->f_path.dentry->d_name structure
    let _ = unsafe { bpf_probe_read_kernel(filename.as_mut_ptr(), 256, file_ptr as *const u8) };
    0
}

fn extract_filename_from_dentry(dentry_ptr: u64, filename: &mut [u8; 256]) -> i32 {
    // Simplified implementation - would parse dentry->d_name in real code
    let _ = unsafe { bpf_probe_read_kernel(filename.as_mut_ptr(), 256, dentry_ptr as *const u8) };
    0
}

fn increment_fs_stat(stat_type: u32) {
    if let Some(count) = unsafe { FS_STATS.get(&stat_type) } {
        let new_count = *count + 1;
        let _ = unsafe { FS_STATS.insert(&stat_type, &new_count, 0) };
    } else {
        let _ = unsafe { FS_STATS.insert(&stat_type, &1u64, 0) };
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
