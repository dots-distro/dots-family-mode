use anyhow::Result;
use dots_family_db::{
    models::{NewDiskIoEvent, NewMemoryEvent},
    Database,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::ebpf::{disk_io_monitor::DiskIoEvent, memory_monitor::MemoryEvent};

/// Event processor for bridging eBPF events to database storage
pub struct EbpfEventProcessor {
    db: Database,
}

impl EbpfEventProcessor {
    /// Create a new event processor
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Process a memory event and store it in the database
    pub async fn process_memory_event(
        &self,
        event: MemoryEvent,
        profile_id: Option<i64>,
    ) -> Result<()> {
        // Get process name from /proc
        let comm = get_process_name(event.pid).unwrap_or_else(|| format!("pid:{}", event.pid));

        // Use provided profile_id or try to determine from active session
        let profile_id = match profile_id {
            Some(id) => id,
            None => {
                // TODO: Look up active profile for this process
                // For now, log and skip events without a profile
                debug!("No active profile for memory event from PID {}, skipping", event.pid);
                return Ok(());
            }
        };

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;

        let new_event = NewMemoryEvent {
            profile_id,
            pid: event.pid as i32,
            comm,
            event_type: event.event_type as i32,
            size: event.size as i64,
            page_order: Some(event.order as i32),
            timestamp,
        };

        // TODO: Call database insert function once integrated
        // dots_family_db::queries::ebpf_metrics::insert_memory_event(&self.db, &new_event).await?;
        debug!("Processed memory event: {:?}", new_event);

        Ok(())
    }

    /// Process a disk I/O event and store it in the database
    pub async fn process_disk_io_event(
        &self,
        event: DiskIoEvent,
        profile_id: Option<i64>,
    ) -> Result<()> {
        // Get process name from /proc
        let comm = get_process_name(event.pid).unwrap_or_else(|| format!("pid:{}", event.pid));

        // Use provided profile_id or try to determine from active session
        let profile_id = match profile_id {
            Some(id) => id,
            None => {
                // TODO: Look up active profile for this process
                // For now, log and skip events without a profile
                debug!("No active profile for disk I/O event from PID {}, skipping", event.pid);
                return Ok(());
            }
        };

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as i64;

        let latency_ns = if event.event_type == 1 {
            // Only complete events have latency
            Some(event.latency as i64)
        } else {
            None
        };

        let new_event = NewDiskIoEvent {
            profile_id,
            pid: event.pid as i32,
            comm,
            device_major: event.device_major() as i32,
            device_minor: event.device_minor() as i32,
            sector: event.sector as i64,
            nr_sectors: event.nr_sector as i32,
            event_type: event.event_type as i32,
            latency_ns,
            timestamp,
        };

        // TODO: Call database insert function once integrated
        // dots_family_db::queries::ebpf_metrics::insert_disk_io_event(&self.db, &new_event).await?;
        debug!("Processed disk I/O event: {:?}", new_event);

        Ok(())
    }

    /// Get the active profile ID from the database
    /// Returns None if no profile is currently active
    #[allow(dead_code)]
    pub async fn get_active_profile_id(&self) -> Result<Option<i64>> {
        // TODO: Implement profile lookup
        // This should query the database for the currently active profile
        // For now, return None
        warn!("Active profile lookup not yet implemented");
        Ok(None)
    }
}

/// Get process name from /proc/[pid]/comm
fn get_process_name(pid: u32) -> Option<String> {
    let comm_path = format!("/proc/{}/comm", pid);
    std::fs::read_to_string(&comm_path).ok().map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_process_name() {
        // Try to get name of init process (PID 1)
        let name = get_process_name(1);
        assert!(name.is_some());
        println!("PID 1 name: {:?}", name);
    }

    #[test]
    fn test_get_process_name_invalid() {
        // Try to get name of non-existent process
        let name = get_process_name(9999999);
        assert!(name.is_none());
    }
}
