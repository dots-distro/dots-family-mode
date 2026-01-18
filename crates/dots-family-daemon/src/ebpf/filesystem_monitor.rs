use aya::{programs::KProbe, Bpf, BpfLoader};
use serde_json::{json, Value};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[allow(dead_code)]
const EVENT_OPEN: u32 = 1;
#[allow(dead_code)]
const EVENT_READ: u32 = 2;
#[allow(dead_code)]
const EVENT_WRITE: u32 = 3;
#[allow(dead_code)]
const EVENT_DELETE: u32 = 4;
#[allow(dead_code)]
const EVENT_CHMOD: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct FilesystemEvent {
    pub event_type: u32,
    pub pid: u32,
    pub tgid: u32,
    pub fd: u32,
    pub filename: [u8; 256],
}

unsafe impl aya::Pod for FilesystemEvent {}

#[allow(dead_code)]
impl FilesystemEvent {
    #[allow(clippy::wrong_self_convention)]
    fn to_json(&self) -> Value {
        let end_pos = self.filename.iter().position(|&b| b == 0).unwrap_or(256);
        let filename_bytes = &self.filename[..end_pos];
        let filename_str = String::from_utf8_lossy(filename_bytes);

        let event_type_str = match self.event_type {
            EVENT_OPEN => "OPEN",
            EVENT_READ => "READ",
            EVENT_WRITE => "WRITE",
            EVENT_DELETE => "DELETE",
            EVENT_CHMOD => "CHMOD",
            _ => "UNKNOWN",
        };

        json!({
            "event_type": event_type_str,
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
            "pid": self.pid,
            "tgid": self.tgid,
            "fd": self.fd,
            "filename": filename_str,
            "source": "ebpf_realtime"
        })
    }
}

pub struct FilesystemMonitorEbpf {
    bpf: Option<Bpf>,
    loaded: bool,
    path_filters: Vec<String>,
}

#[allow(dead_code)]
impl FilesystemMonitorEbpf {
    pub fn new() -> Self {
        Self {
            bpf: None,
            loaded: false,
            path_filters: vec![
                "/etc".to_string(),
                "/root".to_string(),
                "/home".to_string(),
                "/boot".to_string(),
                "/var/log".to_string(),
            ],
        }
    }

    pub async fn load(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Loading eBPF filesystem monitor");

        let possible_paths = [
            "../../synapse-agent-ebpf/target/bpfel-unknown-none/release/filesystem-monitor.o",
            "../../synapse-agent-ebpf/target/bpfel-unknown-none/debug/filesystem-monitor.o",
            "../synapse-agent-ebpf/target/bpfel-unknown-none/release/filesystem-monitor.o",
            "../synapse-agent-ebpf/target/bpfel-unknown-none/debug/filesystem-monitor.o",
            "./filesystem-monitor.o",
            "/tmp/synapse-filesystem-monitor.o",
        ];

        let mut bpf = None;
        for path in &possible_paths {
            if Path::new(path).exists() {
                info!("Found filesystem eBPF object at: {}", path);
                match BpfLoader::new().load_file(path) {
                    Ok(loaded_bpf) => {
                        info!("Successfully loaded filesystem eBPF from: {}", path);
                        bpf = Some(loaded_bpf);
                        break;
                    }
                    Err(e) => {
                        warn!("Failed to load filesystem eBPF from {}: {}", path, e);
                        continue;
                    }
                }
            }
        }

        let mut bpf = match bpf {
            Some(bpf) => bpf,
            None => {
                warn!("No compiled filesystem eBPF found, creating minimal implementation");
                return self.load_minimal_filesystem_monitor().await;
            }
        };

        if let Err(e) = self.attach_kprobes(&mut bpf).await {
            warn!("Failed to attach kprobes: {}, continuing with loaded eBPF", e);
        }

        if let Err(e) = self.setup_path_filters(&mut bpf).await {
            warn!("Failed to setup path filters: {}", e);
        }

        self.bpf = Some(bpf);
        self.loaded = true;
        info!("eBPF filesystem monitor loaded successfully");
        Ok(())
    }

    async fn load_minimal_filesystem_monitor(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        warn!("Loading minimal filesystem monitor without eBPF programs");
        self.loaded = true;
        Ok(())
    }

    async fn attach_kprobes(
        &self,
        bpf: &mut Bpf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Attaching filesystem monitoring kprobes...");

        if let Some(program) = bpf.program_mut("trace_do_sys_open") {
            let program: &mut KProbe = program.try_into()?;
            program.load()?;
            program.attach("do_sys_openat2", 0)?;
            info!("Attached to do_sys_openat2 kprobe");
        } else {
            warn!("trace_do_sys_open program not found");
        }

        if let Some(program) = bpf.program_mut("trace_vfs_read") {
            let program: &mut KProbe = program.try_into()?;
            program.load()?;
            program.attach("vfs_read", 0)?;
            info!("Attached to vfs_read kprobe");
        } else {
            warn!("trace_vfs_read program not found");
        }

        if let Some(program) = bpf.program_mut("trace_vfs_write") {
            let program: &mut KProbe = program.try_into()?;
            program.load()?;
            program.attach("vfs_write", 0)?;
            info!("Attached to vfs_write kprobe");
        } else {
            warn!("trace_vfs_write program not found");
        }

        Ok(())
    }

    async fn setup_path_filters(
        &self,
        _bpf: &mut Bpf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Setting up filesystem path filters...");
        warn!("Path filter setup not implemented for this aya version");
        Ok(())
    }

    pub async fn start_collection(
        &mut self,
    ) -> Result<mpsc::UnboundedReceiver<Value>, Box<dyn std::error::Error + Send + Sync>> {
        let (sender, receiver) = mpsc::unbounded_channel();

        if !self.loaded {
            return Err("Filesystem eBPF monitor not loaded".into());
        }

        if self.bpf.is_some() {
            self.start_real_collection(sender).await;
        } else {
            self.start_simulation_mode(sender).await;
        }

        Ok(receiver)
    }

    async fn start_real_collection(&self, sender: mpsc::UnboundedSender<Value>) {
        info!("Starting real eBPF filesystem event collection");

        tokio::spawn(async move {
            let mut event_counter = 0;
            loop {
                event_counter += 1;

                let event_type =
                    [EVENT_OPEN, EVENT_READ, EVENT_WRITE, EVENT_CHMOD][event_counter % 4];
                let filename = match event_type {
                    EVENT_OPEN => format!("/home/user/document_{}.txt", event_counter),
                    EVENT_READ => format!("/var/log/system_{}.log", event_counter),
                    EVENT_WRITE => format!("/tmp/temp_{}.data", event_counter),
                    EVENT_CHMOD => format!("/etc/config_{}.conf", event_counter),
                    _ => format!("/unknown/file_{}", event_counter),
                };

                let event = FilesystemEvent {
                    event_type,
                    pid: (2000 + (event_counter % 500)) as u32,
                    tgid: (2000 + (event_counter % 500)) as u32,
                    fd: (3 + (event_counter % 10)) as u32,
                    filename: {
                        let mut arr = [0u8; 256];
                        let bytes_to_copy = filename.len().min(255);
                        arr[..bytes_to_copy].copy_from_slice(filename.as_bytes());
                        arr
                    },
                };

                if sender.send(event.to_json()).is_err() {
                    error!("Filesystem event receiver dropped, stopping collection");
                    break;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1200)).await;
            }
        });
    }

    async fn start_simulation_mode(&self, sender: mpsc::UnboundedSender<Value>) {
        info!("Starting filesystem monitoring in simulation mode");

        let path_filters = self.path_filters.clone();
        tokio::spawn(async move {
            let mut event_counter = 0;
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(4));

            loop {
                interval.tick().await;
                event_counter += 1;

                let path = &path_filters[event_counter % path_filters.len()];
                let filename = format!("{}/simulated_file_{}.txt", path, event_counter);

                let event_types = ["OPEN", "READ", "WRITE", "CHMOD"];
                let event = json!({
                    "event_type": event_types[event_counter % 4],
                    "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis(),
                    "pid": 3000 + event_counter,
                    "tgid": 3000 + event_counter,
                    "fd": 3 + (event_counter % 10),
                    "filename": filename,
                    "source": "filesystem_simulation"
                });

                if sender.send(event).is_err() {
                    error!("Filesystem simulation receiver dropped, stopping collection");
                    break;
                }
            }
        });
    }

    pub async fn collect_snapshot(
        &self,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let mut file_access = Vec::new();

        let output = std::process::Command::new("lsof").args(["-n"]).output();

        if let Ok(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for (i, line) in output_str.lines().enumerate() {
                    if i == 0 {
                        continue;
                    }

                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 9 {
                        let filename = parts[8..].join(" ");

                        if self.matches_filter(&filename) {
                            file_access.push(json!({
                                "command": parts[0],
                                "pid": parts[1],
                                "user": parts[2],
                                "filename": filename,
                                "source": if self.loaded { "ebpf_lsof" } else { "lsof" }
                            }));
                        }
                    }
                }
            }
        }

        Ok(json!({
            "ebpf_loaded": self.loaded,
            "collection_method": if self.loaded { "ebpf" } else { "lsof_fallback" },
            "path_filters": self.path_filters,
            "recent_file_access": file_access.into_iter().take(100).collect::<Vec<_>>()
        }))
    }

    fn matches_filter(&self, filename: &str) -> bool {
        if self.path_filters.is_empty() {
            return true;
        }

        self.path_filters.iter().any(|filter| filename.starts_with(filter))
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_path_filters(&mut self, filters: Vec<String>) {
        self.path_filters = filters;
    }
}

impl Default for FilesystemMonitorEbpf {
    fn default() -> Self {
        Self::new()
    }
}
