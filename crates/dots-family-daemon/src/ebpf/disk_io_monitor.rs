use anyhow::Result;
use tracing::{debug, info};

/// Disk I/O event from eBPF program
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DiskIoEvent {
    pub pid: u32,
    pub dev: u32,       // Device ID (major:minor)
    pub sector: u64,    // Starting sector
    pub nr_sector: u32, // Number of sectors
    pub latency: u64,   // I/O latency in nanoseconds (for complete events)
    pub event_type: u8, // 0=issue, 1=complete, 2=bio_queue
}

impl DiskIoEvent {
    /// Get device major number
    pub fn device_major(&self) -> u32 {
        (self.dev >> 20) & 0xfff
    }

    /// Get device minor number
    pub fn device_minor(&self) -> u32 {
        self.dev & 0xfffff
    }

    /// Get I/O size in bytes (assuming 512-byte sectors)
    pub fn io_size_bytes(&self) -> u64 {
        self.nr_sector as u64 * 512
    }

    /// Get latency in microseconds
    pub fn latency_us(&self) -> f64 {
        self.latency as f64 / 1000.0
    }

    /// Get latency in milliseconds
    pub fn latency_ms(&self) -> f64 {
        self.latency as f64 / 1_000_000.0
    }

    /// Get event type name
    pub fn event_type_name(&self) -> &'static str {
        match self.event_type {
            0 => "issue",
            1 => "complete",
            2 => "bio_queue",
            _ => "unknown",
        }
    }
}

/// Disk I/O monitor for eBPF-based I/O tracking
pub struct DiskIoMonitorEbpf {
    _marker: std::marker::PhantomData<()>,
}

impl DiskIoMonitorEbpf {
    /// Create a new disk I/O monitor instance
    pub fn new() -> Result<Self> {
        info!("Initializing eBPF disk I/O monitor");
        Ok(Self { _marker: std::marker::PhantomData })
    }

    /// Process a disk I/O event from the eBPF program
    pub async fn process_event(&self, event: DiskIoEvent) -> Result<()> {
        debug!(
            "Disk I/O event: pid={} dev={}:{} type={} size={} bytes latency={:.2} ms",
            event.pid,
            event.device_major(),
            event.device_minor(),
            event.event_type_name(),
            event.io_size_bytes(),
            event.latency_ms()
        );

        // TODO: Store event in database
        // TODO: Track latency metrics
        // TODO: Detect excessive I/O
        // TODO: Alert on high latency

        Ok(())
    }

    /// Get I/O statistics for a process
    #[allow(dead_code)]
    pub async fn get_process_io_stats(&self, _pid: u32) -> Result<DiskIoStats> {
        // TODO: Implement actual statistics gathering
        Ok(DiskIoStats {
            read_bytes: 0,
            write_bytes: 0,
            read_count: 0,
            write_count: 0,
            avg_latency_ms: 0.0,
        })
    }
}

/// Disk I/O statistics for a process
#[derive(Debug, Clone)]
pub struct DiskIoStats {
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_count: u64,
    pub write_count: u64,
    pub avg_latency_ms: f64,
}
