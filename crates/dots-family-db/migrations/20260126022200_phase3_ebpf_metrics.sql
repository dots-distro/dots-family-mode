-- Phase 3 eBPF Metrics Tables
-- Add support for memory monitoring and disk I/O tracking

-- Memory Events Table
-- Tracks kernel memory allocations and page operations
CREATE TABLE IF NOT EXISTS memory_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    event_type INTEGER NOT NULL, -- 0=kmalloc, 1=kfree, 2=page_alloc, 3=page_free
    size INTEGER NOT NULL,        -- Size in bytes
    page_order INTEGER,           -- Page order (for page events)
    timestamp INTEGER NOT NULL,   -- Unix timestamp in milliseconds
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_memory_events_profile_timestamp ON memory_events(profile_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_memory_events_pid ON memory_events(pid);
CREATE INDEX IF NOT EXISTS idx_memory_events_timestamp ON memory_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_memory_events_type ON memory_events(event_type);

-- Disk I/O Events Table  
-- Tracks block device I/O operations with latency
CREATE TABLE IF NOT EXISTS disk_io_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    device_major INTEGER NOT NULL, -- Device major number
    device_minor INTEGER NOT NULL, -- Device minor number
    sector INTEGER NOT NULL,       -- Starting sector
    nr_sectors INTEGER NOT NULL,   -- Number of sectors
    event_type INTEGER NOT NULL,   -- 0=issue, 1=complete, 2=bio_queue
    latency_ns INTEGER,            -- I/O latency in nanoseconds (for complete events)
    timestamp INTEGER NOT NULL,    -- Unix timestamp in milliseconds
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_disk_io_events_profile_timestamp ON disk_io_events(profile_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_pid ON disk_io_events(pid);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_timestamp ON disk_io_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_device ON disk_io_events(device_major, device_minor);
CREATE INDEX IF NOT EXISTS idx_disk_io_events_type ON disk_io_events(event_type);

-- Network Activity Enhancement
-- Add bandwidth columns to existing network_activity table if not present
-- This supports the enhanced network-monitor with tcp_sendmsg/tcp_recvmsg

-- Check if we need to add columns (SQLite doesn't have IF NOT EXISTS for columns)
-- We'll use a separate migration if the table needs modification

-- Memory Statistics Aggregation Table
-- Pre-computed statistics for efficient querying
CREATE TABLE IF NOT EXISTS memory_stats_hourly (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    hour_timestamp INTEGER NOT NULL, -- Start of hour (Unix timestamp)
    total_allocated_bytes INTEGER NOT NULL DEFAULT 0,
    total_freed_bytes INTEGER NOT NULL DEFAULT 0,
    net_allocation_bytes INTEGER NOT NULL DEFAULT 0,
    peak_allocation_bytes INTEGER NOT NULL DEFAULT 0,
    allocation_count INTEGER NOT NULL DEFAULT 0,
    free_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, pid, hour_timestamp)
);

CREATE INDEX IF NOT EXISTS idx_memory_stats_profile_hour ON memory_stats_hourly(profile_id, hour_timestamp);
CREATE INDEX IF NOT EXISTS idx_memory_stats_pid ON memory_stats_hourly(pid);

-- Disk I/O Statistics Aggregation Table
-- Pre-computed statistics for efficient querying
CREATE TABLE IF NOT EXISTS disk_io_stats_hourly (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    profile_id INTEGER NOT NULL,
    pid INTEGER NOT NULL,
    comm TEXT NOT NULL,
    device_major INTEGER NOT NULL,
    device_minor INTEGER NOT NULL,
    hour_timestamp INTEGER NOT NULL, -- Start of hour (Unix timestamp)
    total_read_bytes INTEGER NOT NULL DEFAULT 0,
    total_write_bytes INTEGER NOT NULL DEFAULT 0,
    read_count INTEGER NOT NULL DEFAULT 0,
    write_count INTEGER NOT NULL DEFAULT 0,
    total_latency_ns INTEGER NOT NULL DEFAULT 0,
    min_latency_ns INTEGER,
    max_latency_ns INTEGER,
    avg_latency_ns INTEGER,
    FOREIGN KEY (profile_id) REFERENCES profiles(id) ON DELETE CASCADE,
    UNIQUE(profile_id, pid, device_major, device_minor, hour_timestamp)
);

CREATE INDEX IF NOT EXISTS idx_disk_io_stats_profile_hour ON disk_io_stats_hourly(profile_id, hour_timestamp);
CREATE INDEX IF NOT EXISTS idx_disk_io_stats_pid ON disk_io_stats_hourly(pid);
CREATE INDEX IF NOT EXISTS idx_disk_io_stats_device ON disk_io_stats_hourly(device_major, device_minor);
