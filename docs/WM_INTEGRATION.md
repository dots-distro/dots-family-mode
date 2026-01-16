# Window Manager Integration

## Overview

Family Mode supports three Wayland compositors through a unified bridge architecture: Niri (Rust), Swayfx (C++), and Hyprland (C++). Each WM provides different capabilities for window tracking and control.

## Architecture

```
┌────────────────────────────────────────────────────────────┐
│              dots-family-monitor                            │
└─────────────────────┬──────────────────────────────────────┘
                      │
                      │ Unified Window API
                      │
┌─────────────────────▼──────────────────────────────────────┐
│              dots-wm-bridge                                 │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │         WindowManagerAdapter Trait                    │  │
│  │  - get_active_window()                                │  │
│  │  - list_windows()                                     │  │
│  │  - close_window()                                     │  │
│  │  - subscribe_events()                                 │  │
│  └──────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌──────────┐    ┌───────────┐    ┌──────────────┐        │
│  │   Niri   │    │  Swayfx   │    │  Hyprland    │        │
│  │ Adapter  │    │  Adapter  │    │   Adapter    │        │
│  └─────┬────┘    └─────┬─────┘    └──────┬───────┘        │
└────────┼───────────────┼──────────────────┼────────────────┘
         │               │                  │
┌────────▼───────┐ ┌────▼──────┐ ┌─────────▼──────┐
│  Niri IPC      │ │ Sway IPC  │ │ Hyprland       │
│  (Unix Socket) │ │ (JSON)    │ │ Socket         │
└────────────────┘ └───────────┘ └────────────────┘
```

## Unified Window API

### Window Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    /// Unique identifier for the window
    pub id: String,

    /// Window title
    pub title: String,

    /// Application ID (Wayland app_id or X11 class)
    pub app_id: String,

    /// Human-readable application name
    pub app_name: String,

    /// Workspace/output where window is located
    pub workspace: Option<String>,

    /// Whether window is currently focused
    pub is_focused: bool,

    /// Process ID of the application
    pub pid: Option<u32>,

    /// Window geometry
    pub geometry: WindowGeometry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
```

### WindowManagerAdapter Trait

```rust
use async_trait::async_trait;
use futures::stream::Stream;

#[async_trait]
pub trait WindowManagerAdapter: Send + Sync {
    /// Get the currently active (focused) window
    async fn get_active_window(&self) -> Result<Window>;

    /// List all windows
    async fn list_windows(&self) -> Result<Vec<Window>>;

    /// Get window by ID
    async fn get_window(&self, id: &str) -> Result<Option<Window>>;

    /// Close a window
    async fn close_window(&self, id: &str) -> Result<()>;

    /// Focus a window
    async fn focus_window(&self, id: &str) -> Result<()>;

    /// Subscribe to window events
    async fn subscribe_events(&self) -> Result<WindowEventStream>;

    /// Get window manager name and version
    fn wm_info(&self) -> WMInfo;
}

pub type WindowEventStream = Pin<Box<dyn Stream<Item = WindowEvent> + Send>>;

#[derive(Debug, Clone)]
pub enum WindowEvent {
    WindowOpened(Window),
    WindowClosed { id: String },
    WindowFocused { id: String },
    WindowUnfocused { id: String },
    WindowTitleChanged { id: String, new_title: String },
    WorkspaceChanged { name: String },
}

#[derive(Debug, Clone)]
pub struct WMInfo {
    pub name: String,
    pub version: String,
    pub capabilities: WMCapabilities,
}

#[derive(Debug, Clone)]
pub struct WMCapabilities {
    pub window_events: bool,
    pub close_window: bool,
    pub focus_window: bool,
    pub workspace_info: bool,
    pub process_info: bool,
}
```

## Niri Integration

### IPC Protocol

Niri provides a Rust IPC library for communication.

**Dependencies**:
```toml
[dependencies]
niri-ipc = { git = "https://github.com/YaLTeR/niri" }
tokio = { version = "1.35", features = ["net", "io-util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Niri Adapter Implementation

```rust
use niri_ipc::{Socket, Request, Response, Event};
use std::path::PathBuf;

pub struct NiriAdapter {
    socket: Socket,
    socket_path: PathBuf,
}

impl NiriAdapter {
    pub async fn new() -> Result<Self> {
        let socket_path = Self::find_socket_path()?;
        let socket = Socket::connect(&socket_path).await?;

        Ok(Self {
            socket,
            socket_path,
        })
    }

    fn find_socket_path() -> Result<PathBuf> {
        // Niri socket location: $XDG_RUNTIME_DIR/niri/niri.sock
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map_err(|_| anyhow!("XDG_RUNTIME_DIR not set"))?;

        let socket_path = PathBuf::from(runtime_dir)
            .join("niri")
            .join("niri.sock");

        if !socket_path.exists() {
            return Err(anyhow!("Niri socket not found"));
        }

        Ok(socket_path)
    }
}

#[async_trait]
impl WindowManagerAdapter for NiriAdapter {
    async fn get_active_window(&self) -> Result<Window> {
        let response = self.socket.send(Request::Windows).await?;

        match response {
            Response::Windows(windows) => {
                let active = windows
                    .iter()
                    .find(|w| w.is_focused)
                    .ok_or_else(|| anyhow!("No active window"))?;

                Ok(self.convert_window(active))
            }
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    async fn list_windows(&self) -> Result<Vec<Window>> {
        let response = self.socket.send(Request::Windows).await?;

        match response {
            Response::Windows(windows) => {
                Ok(windows.iter().map(|w| self.convert_window(w)).collect())
            }
            _ => Err(anyhow!("Unexpected response type")),
        }
    }

    async fn close_window(&self, id: &str) -> Result<()> {
        let window_id = id.parse::<u64>()?;

        // Niri IPC action to close window
        let request = Request::Action {
            action: niri_ipc::Action::CloseWindow { window_id },
        };

        self.socket.send(request).await?;
        Ok(())
    }

    async fn subscribe_events(&self) -> Result<WindowEventStream> {
        let event_stream = self.socket.subscribe_events().await?;

        // Convert Niri events to our WindowEvent
        let stream = event_stream.map(|event| {
            match event {
                Event::WindowOpened { window } => {
                    WindowEvent::WindowOpened(self.convert_window(&window))
                }
                Event::WindowClosed { window_id } => {
                    WindowEvent::WindowClosed {
                        id: window_id.to_string(),
                    }
                }
                Event::WindowFocused { window_id } => {
                    WindowEvent::WindowFocused {
                        id: window_id.to_string(),
                    }
                }
                // ... other events
            }
        });

        Ok(Box::pin(stream))
    }

    fn wm_info(&self) -> WMInfo {
        WMInfo {
            name: "Niri".to_string(),
            version: env!("NIRI_VERSION").to_string(),
            capabilities: WMCapabilities {
                window_events: true,
                close_window: true,
                focus_window: true,
                workspace_info: true,
                process_info: true,
            },
        }
    }
}

impl NiriAdapter {
    fn convert_window(&self, niri_window: &niri_ipc::Window) -> Window {
        Window {
            id: niri_window.id.to_string(),
            title: niri_window.title.clone(),
            app_id: niri_window.app_id.clone(),
            app_name: self.lookup_app_name(&niri_window.app_id),
            workspace: Some(niri_window.workspace_id.to_string()),
            is_focused: niri_window.is_focused,
            pid: Some(niri_window.pid),
            geometry: WindowGeometry {
                x: niri_window.geometry.x,
                y: niri_window.geometry.y,
                width: niri_window.geometry.width as u32,
                height: niri_window.geometry.height as u32,
            },
        }
    }

    fn lookup_app_name(&self, app_id: &str) -> String {
        // Look up desktop file for human-readable name
        if let Ok(desktop_entry) = freedesktop_entry_parser::parse_entry(
            format!("/usr/share/applications/{}.desktop", app_id)
        ) {
            if let Some(name) = desktop_entry.section("Desktop Entry")
                .attr("Name") {
                return name.to_string();
            }
        }

        // Fallback to app_id
        app_id.to_string()
    }
}
```

### Niri Event Subscription

```rust
pub async fn listen_niri_events(
    adapter: &NiriAdapter,
    callback: impl Fn(WindowEvent),
) -> Result<()> {
    let mut stream = adapter.subscribe_events().await?;

    while let Some(event) = stream.next().await {
        callback(event);
    }

    Ok(())
}
```

### Niri-Specific Features

**Advantages**:
- Native Rust IPC (type-safe, efficient)
- Real-time event stream
- Complete window information including PID
- No JSON parsing overhead

**Limitations**:
- Niri-specific, not compatible with other WMs

## Swayfx Integration

### IPC Protocol

Swayfx is compatible with Sway's IPC protocol (JSON over Unix socket).

**Dependencies**:
```toml
[dependencies]
tokio = { version = "1.35", features = ["net", "io-util"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Sway IPC Messages

**Message Types**:
```rust
#[repr(u32)]
enum SwayMessageType {
    RunCommand = 0,
    GetWorkspaces = 1,
    Subscribe = 2,
    GetOutputs = 3,
    GetTree = 4,
    GetMarks = 5,
    GetBarConfig = 6,
    GetVersion = 7,
    GetBindingModes = 8,
    GetConfig = 9,
    SendTick = 10,
    Sync = 11,
    GetBindingState = 12,
    GetInputs = 100,
    GetSeats = 101,
}
```

**Message Format**:
```
┌─────────────┬───────────────┬──────────────┐
│ Magic       │ Payload Len   │ Message Type │ Payload (JSON)
│ (i3-ipc)    │ (u32 LE)      │ (u32 LE)     │
│ 6 bytes     │ 4 bytes       │ 4 bytes      │ variable
└─────────────┴───────────────┴──────────────┘
```

### Swayfx Adapter Implementation

```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct SwayfxAdapter {
    socket_path: PathBuf,
}

impl SwayfxAdapter {
    pub async fn new() -> Result<Self> {
        let socket_path = Self::find_socket_path()?;
        Ok(Self { socket_path })
    }

    fn find_socket_path() -> Result<PathBuf> {
        // Sway socket: $SWAYSOCK or $I3SOCK
        if let Ok(socket) = std::env::var("SWAYSOCK") {
            return Ok(PathBuf::from(socket));
        }

        if let Ok(socket) = std::env::var("I3SOCK") {
            return Ok(PathBuf::from(socket));
        }

        Err(anyhow!("Sway socket not found"))
    }

    async fn send_message(
        &self,
        msg_type: SwayMessageType,
        payload: &str,
    ) -> Result<serde_json::Value> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;

        // Build message
        let magic = b"i3-ipc";
        let payload_len = payload.len() as u32;
        let msg_type = msg_type as u32;

        // Write header
        stream.write_all(magic).await?;
        stream.write_u32_le(payload_len).await?;
        stream.write_u32_le(msg_type).await?;

        // Write payload
        stream.write_all(payload.as_bytes()).await?;

        // Read response header
        let mut magic_buf = [0u8; 6];
        stream.read_exact(&mut magic_buf).await?;
        assert_eq!(&magic_buf, b"i3-ipc");

        let response_len = stream.read_u32_le().await?;
        let _response_type = stream.read_u32_le().await?;

        // Read response payload
        let mut payload_buf = vec![0u8; response_len as usize];
        stream.read_exact(&mut payload_buf).await?;

        // Parse JSON
        let response: serde_json::Value = serde_json::from_slice(&payload_buf)?;
        Ok(response)
    }
}

#[async_trait]
impl WindowManagerAdapter for SwayfxAdapter {
    async fn get_active_window(&self) -> Result<Window> {
        // Get window tree
        let tree = self.send_message(
            SwayMessageType::GetTree,
            ""
        ).await?;

        // Find focused window
        let focused = Self::find_focused_node(&tree)
            .ok_or_else(|| anyhow!("No focused window"))?;

        Ok(self.parse_window(focused)?)
    }

    async fn list_windows(&self) -> Result<Vec<Window>> {
        let tree = self.send_message(
            SwayMessageType::GetTree,
            ""
        ).await?;

        let windows = Self::extract_windows(&tree);
        Ok(windows.into_iter()
            .map(|node| self.parse_window(node))
            .collect::<Result<Vec<_>>>()?)
    }

    async fn close_window(&self, id: &str) -> Result<()> {
        // Use Sway command to close window
        let command = format!("[con_id={}] kill", id);
        self.send_message(
            SwayMessageType::RunCommand,
            &command
        ).await?;

        Ok(())
    }

    async fn subscribe_events(&self) -> Result<WindowEventStream> {
        // Subscribe to window events
        let payload = r#"["window", "workspace"]"#;

        let mut stream = UnixStream::connect(&self.socket_path).await?;

        // Send subscription message
        let magic = b"i3-ipc";
        stream.write_all(magic).await?;
        stream.write_u32_le(payload.len() as u32).await?;
        stream.write_u32_le(SwayMessageType::Subscribe as u32).await?;
        stream.write_all(payload.as_bytes()).await?;

        // Create event stream
        let event_stream = Self::create_event_stream(stream);
        Ok(Box::pin(event_stream))
    }

    fn wm_info(&self) -> WMInfo {
        WMInfo {
            name: "Swayfx".to_string(),
            version: "0.3.0".to_string(), // TODO: get from GetVersion
            capabilities: WMCapabilities {
                window_events: true,
                close_window: true,
                focus_window: true,
                workspace_info: true,
                process_info: true,
            },
        }
    }
}

impl SwayfxAdapter {
    fn find_focused_node(node: &serde_json::Value) -> Option<&serde_json::Value> {
        if node["focused"].as_bool() == Some(true) {
            return Some(node);
        }

        if let Some(nodes) = node["nodes"].as_array() {
            for child in nodes {
                if let Some(focused) = Self::find_focused_node(child) {
                    return Some(focused);
                }
            }
        }

        if let Some(floating) = node["floating_nodes"].as_array() {
            for child in floating {
                if let Some(focused) = Self::find_focused_node(child) {
                    return Some(focused);
                }
            }
        }

        None
    }

    fn parse_window(&self, node: &serde_json::Value) -> Result<Window> {
        Ok(Window {
            id: node["id"].as_u64()
                .ok_or_else(|| anyhow!("Missing id"))?
                .to_string(),
            title: node["name"].as_str()
                .unwrap_or("")
                .to_string(),
            app_id: node["app_id"].as_str()
                .or_else(|| node["window_properties"]["class"].as_str())
                .unwrap_or("unknown")
                .to_string(),
            app_name: node["app_id"].as_str()
                .unwrap_or("unknown")
                .to_string(),
            workspace: node["workspace"].as_str()
                .map(|s| s.to_string()),
            is_focused: node["focused"].as_bool()
                .unwrap_or(false),
            pid: node["pid"].as_u64()
                .map(|p| p as u32),
            geometry: WindowGeometry {
                x: node["rect"]["x"].as_i64().unwrap_or(0) as i32,
                y: node["rect"]["y"].as_i64().unwrap_or(0) as i32,
                width: node["rect"]["width"].as_u64().unwrap_or(0) as u32,
                height: node["rect"]["height"].as_u64().unwrap_or(0) as u32,
            },
        })
    }
}
```

### Swayfx-Specific Features

**Advantages**:
- Compatible with Sway ecosystem
- Well-documented IPC protocol
- Complete window information

**Limitations**:
- JSON parsing overhead
- Must maintain socket connection for events
- No type safety (JSON)

## Hyprland Integration

### IPC Protocol

Hyprland provides two sockets: one for commands, one for events.

**Socket Locations**:
- Commands: `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock`
- Events: `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock`

### Hyprland Adapter Implementation

```rust
use tokio::net::UnixStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct HyprlandAdapter {
    command_socket: PathBuf,
    event_socket: PathBuf,
}

impl HyprlandAdapter {
    pub async fn new() -> Result<Self> {
        let (command_socket, event_socket) = Self::find_sockets()?;

        Ok(Self {
            command_socket,
            event_socket,
        })
    }

    fn find_sockets() -> Result<(PathBuf, PathBuf)> {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")?;
        let signature = std::env::var("HYPRLAND_INSTANCE_SIGNATURE")?;

        let base = PathBuf::from(runtime_dir)
            .join("hypr")
            .join(signature);

        Ok((
            base.join(".socket.sock"),
            base.join(".socket2.sock"),
        ))
    }

    async fn send_command(&self, command: &str) -> Result<String> {
        let mut stream = UnixStream::connect(&self.command_socket).await?;

        stream.write_all(command.as_bytes()).await?;
        stream.shutdown().await?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;

        Ok(response)
    }
}

#[async_trait]
impl WindowManagerAdapter for HyprlandAdapter {
    async fn get_active_window(&self) -> Result<Window> {
        let response = self.send_command("j/activewindow").await?;
        let data: serde_json::Value = serde_json::from_str(&response)?;

        self.parse_window(&data)
    }

    async fn list_windows(&self) -> Result<Vec<Window>> {
        let response = self.send_command("j/clients").await?;
        let data: serde_json::Value = serde_json::from_str(&response)?;

        let windows = data.as_array()
            .ok_or_else(|| anyhow!("Expected array"))?;

        windows.iter()
            .map(|w| self.parse_window(w))
            .collect()
    }

    async fn close_window(&self, id: &str) -> Result<()> {
        let command = format!("dispatch closewindow address:{}", id);
        self.send_command(&command).await?;
        Ok(())
    }

    async fn subscribe_events(&self) -> Result<WindowEventStream> {
        let stream = UnixStream::connect(&self.event_socket).await?;
        let reader = BufReader::new(stream);

        let event_stream = reader.lines()
            .filter_map(|line| async move {
                let line = line.ok()?;
                Self::parse_event(&line)
            });

        Ok(Box::pin(event_stream))
    }

    fn wm_info(&self) -> WMInfo {
        WMInfo {
            name: "Hyprland".to_string(),
            version: "0.35.0".to_string(), // TODO: get from version command
            capabilities: WMCapabilities {
                window_events: true,
                close_window: true,
                focus_window: true,
                workspace_info: true,
                process_info: true,
            },
        }
    }
}

impl HyprlandAdapter {
    fn parse_window(&self, data: &serde_json::Value) -> Result<Window> {
        Ok(Window {
            id: data["address"].as_str()
                .ok_or_else(|| anyhow!("Missing address"))?
                .to_string(),
            title: data["title"].as_str()
                .unwrap_or("")
                .to_string(),
            app_id: data["class"].as_str()
                .unwrap_or("unknown")
                .to_string(),
            app_name: data["class"].as_str()
                .unwrap_or("unknown")
                .to_string(),
            workspace: Some(data["workspace"]["name"].as_str()
                .unwrap_or("")
                .to_string()),
            is_focused: data["focusHistoryID"].as_i64() == Some(0),
            pid: data["pid"].as_i64()
                .map(|p| p as u32),
            geometry: WindowGeometry {
                x: data["at"][0].as_i64().unwrap_or(0) as i32,
                y: data["at"][1].as_i64().unwrap_or(0) as i32,
                width: data["size"][0].as_u64().unwrap_or(0) as u32,
                height: data["size"][1].as_u64().unwrap_or(0) as u32,
            },
        })
    }

    fn parse_event(line: &str) -> Option<WindowEvent> {
        // Event format: "eventname>>data"
        let parts: Vec<&str> = line.splitn(2, ">>").collect();
        if parts.len() != 2 {
            return None;
        }

        match parts[0] {
            "openwindow" => {
                // Format: "address,workspace,class,title"
                let fields: Vec<&str> = parts[1].split(',').collect();
                // Parse and create WindowEvent::WindowOpened
                // ...
                None // TODO: implement
            }
            "closewindow" => {
                Some(WindowEvent::WindowClosed {
                    id: parts[1].to_string(),
                })
            }
            "activewindow" => {
                Some(WindowEvent::WindowFocused {
                    id: parts[1].split(',').next()?.to_string(),
                })
            }
            _ => None,
        }
    }
}
```

### Hyprland-Specific Features

**Advantages**:
- Simple text-based protocol
- Separate event socket
- Rich window information

**Limitations**:
- Text parsing required
- Event format not JSON
- Less structured than Niri

## Automatic WM Detection

```rust
pub async fn detect_window_manager() -> Result<Box<dyn WindowManagerAdapter>> {
    // Check environment variables
    if std::env::var("NIRI_SOCKET").is_ok() {
        return Ok(Box::new(NiriAdapter::new().await?));
    }

    if std::env::var("SWAYSOCK").is_ok() || std::env::var("I3SOCK").is_ok() {
        return Ok(Box::new(SwayfxAdapter::new().await?));
    }

    if std::env::var("HYPRLAND_INSTANCE_SIGNATURE").is_ok() {
        return Ok(Box::new(HyprlandAdapter::new().await?));
    }

    // Try each adapter
    if let Ok(adapter) = NiriAdapter::new().await {
        return Ok(Box::new(adapter));
    }

    if let Ok(adapter) = SwayfxAdapter::new().await {
        return Ok(Box::new(adapter));
    }

    if let Ok(adapter) = HyprlandAdapter::new().await {
        return Ok(Box::new(adapter));
    }

    Err(anyhow!("No supported window manager detected"))
}
```

## Fallback: Generic Wayland

For unsupported WMs, use generic Wayland protocols.

```rust
use wayland_client::{Connection, Proxy};
use wayland_protocols::wlr::foreign_toplevel::v1::client::*;

pub struct GenericWaylandAdapter {
    conn: Connection,
}

// Limited functionality:
// - List windows via foreign-toplevel protocol
// - No window control
// - No events
```

## Performance Considerations

### Polling vs Events

**Polling** (get_active_window in loop):
- Pros: Simple, works with all WMs
- Cons: Higher CPU usage, delayed detection

**Events** (subscribe_events):
- Pros: Immediate, low CPU
- Cons: Must maintain connection, more complex

### Caching

Cache window information to reduce IPC calls:

```rust
pub struct CachedAdapter {
    adapter: Box<dyn WindowManagerAdapter>,
    cache: Arc<RwLock<WindowCache>>,
    ttl: Duration,
}

impl CachedAdapter {
    pub async fn get_active_window(&self) -> Result<Window> {
        let cache = self.cache.read().await;

        if let Some((window, timestamp)) = &cache.active {
            if timestamp.elapsed() < self.ttl {
                return Ok(window.clone());
            }
        }

        drop(cache);

        // Cache miss, fetch from adapter
        let window = self.adapter.get_active_window().await?;

        let mut cache = self.cache.write().await;
        cache.active = Some((window.clone(), Instant::now()));

        Ok(window)
    }
}
```

## Testing

### Mock Adapter

```rust
pub struct MockAdapter {
    windows: Vec<Window>,
    active: usize,
}

#[async_trait]
impl WindowManagerAdapter for MockAdapter {
    async fn get_active_window(&self) -> Result<Window> {
        Ok(self.windows[self.active].clone())
    }

    async fn list_windows(&self) -> Result<Vec<Window>> {
        Ok(self.windows.clone())
    }

    // ... other methods
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_niri_adapter() {
    // Requires Niri running
    if std::env::var("NIRI_SOCKET").is_err() {
        return; // Skip if Niri not available
    }

    let adapter = NiriAdapter::new().await.unwrap();
    let window = adapter.get_active_window().await.unwrap();

    assert!(!window.id.is_empty());
    assert!(!window.app_id.is_empty());
}
```

## Related Documentation

- ARCHITECTURE.md: Overall system design
- RUST_APPLICATIONS.md: WM bridge application details
- MONITORING.md: How window data is used
