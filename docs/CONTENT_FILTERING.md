# Content Filtering System

## Overview

The Family Mode content filtering system provides multi-layered protection against inappropriate content across web browsing, application content, and terminal commands. Built in Rust for performance and security, it operates locally without cloud dependencies while maintaining comprehensive coverage.

## Filtering Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                        Content Sources                        │
├──────────────┬──────────────┬──────────────┬─────────────────┤
│ Web Browser  │ Applications │   Terminal   │  File System    │
└──────┬───────┴──────┬───────┴──────┬───────┴─────────┬───────┘
       │              │              │                 │
       │ HTTP/HTTPS   │ IPC/DBus     │ PTY             │ inotify
       │              │              │                 │
┌──────▼──────────────▼──────────────▼─────────────────▼───────┐
│               dots-family-filter (Rust)                       │
│                                                                │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐         │
│  │ Web Filter  │  │ App Filter  │  │ Term Filter  │         │
│  │ (Proxy)     │  │ (Content)   │  │ (Commands)   │         │
│  └──────┬──────┘  └──────┬──────┘  └──────┬───────┘         │
│         │                │                │                   │
│         └────────────────┼────────────────┘                   │
│                          │                                    │
│                   ┌──────▼──────┐                             │
│                   │ Filter      │                             │
│                   │ Engine      │                             │
│                   │             │                             │
│                   │ - URL match │                             │
│                   │ - Category  │                             │
│                   │ - Pattern   │                             │
│                   │ - ML detect │                             │
│                   └──────┬──────┘                             │
│                          │                                    │
│                   ┌──────▼──────┐                             │
│                   │ Filter      │                             │
│                   │ Lists       │                             │
│                   │             │                             │
│                   │ - Blocklist │                             │
│                   │ - Categories│                             │
│                   │ - Patterns  │                             │
│                   └─────────────┘                             │
└────────────────────────────────────────────────────────────────┘
```

## Web Content Filtering

### HTTP/HTTPS Proxy

**Purpose**: Intercept and filter all web traffic

**Implementation**:
- Transparent HTTP proxy on localhost
- HTTPS inspection via parent-authorized certificate (optional)
- DNS-based filtering fallback
- Per-profile proxy configuration

**Configuration**:
```toml
[web_filtering]
enabled = true
mode = "proxy" # or "dns" or "hybrid"

[web_filtering.proxy]
listen_address = "127.0.0.1"
listen_port = 8118
upstream_proxy = "" # Optional upstream proxy

# HTTPS inspection (requires parent consent)
https_inspection = false
certificate_path = "~/.config/dots-family/proxy-cert.pem"
```

**Proxy Features**:
- No performance impact for allowed sites
- Real-time threat detection
- Content pattern matching
- Safe search enforcement
- QUIC/HTTP3 support

### Filter Lists

**Format**: AdBlock Plus compatible syntax

**List Sources**:
```toml
[web_filtering.lists]
# Built-in lists (bundled with Family Mode)
builtin = [
  "family-mode-base",      # Core blocking rules
  "family-mode-adult",     # Adult content
  "family-mode-violence",  # Violence/gore
  "family-mode-gambling",  # Gambling sites
  "family-mode-social",    # Social media
]

# Community lists (auto-updated)
community = [
  "easylist",
  "easyprivacy",
  "fanboy-annoyances"
]

# Custom lists
custom = [
  "file:///etc/dots-family/custom-filters.txt",
  "https://family.example.com/school-filters.txt"
]

# Update schedule
auto_update = true
update_interval_hours = 24
update_on_metered = false
```

**Filter List Format**:
```
! Family Mode Custom Filters
! Title: School Custom Filters
! Homepage: https://school.example.com
! Expires: 1 day

! Block specific domains
||inappropriate-site.com^
||bad-domain.net^

! Block URL patterns
/ads/banners/*
/track/analytics/*

! Category-based (custom extension)
@@||educational-site.com^$category=education
||gaming-site.com^$category=games

! Time-based (custom extension)
||social-media.com^$time=weekday
@@||social-media.com^$time=weekend-16:00-18:00
```

**Filter Matching Performance**:
- Bloom filter for fast negative lookups (O(1))
- Trie structure for domain matching (O(log n))
- Regex patterns cached and compiled
- Expected latency: <1ms per request

### Category-Based Filtering

**Categories**:
```rust
pub enum ContentCategory {
    // Safety categories
    Adult,
    Violence,
    Gambling,
    Drugs,
    Weapons,
    Hate,

    // Activity categories
    SocialMedia,
    Gaming,
    VideoStreaming,
    Shopping,
    News,

    // Productivity categories
    Education,
    Reference,
    Development,
    Finance,

    // Uncategorized
    Unknown,
}
```

**Configuration**:
```toml
[web_filtering.categories]
blocked = [
  "adult",
  "violence",
  "gambling",
  "drugs",
  "hate"
]

restricted = [
  { category = "social-media", allowed_time = "weekends", max_daily_minutes = 60 },
  { category = "gaming", allowed_time = "16:00-18:00", max_daily_minutes = 60 },
  { category = "video-streaming", allowed_sites = ["khanacademy.org", "ted.com"] }
]

always_allowed = [
  "education",
  "reference",
  "development"
]
```

**Category Detection**:
1. **Domain-based**: Pre-classified domain list
2. **Content analysis**: Page text and metadata analysis
3. **ML classification**: Local neural network model (optional)
4. **Heuristics**: URL patterns, keywords, page structure
5. **Manual override**: Parent classification

**ML Model** (optional, privacy-preserving):
```toml
[web_filtering.ml]
enabled = false # Disabled by default for performance
model_path = "~/.local/share/dots-family/models/content-classifier.onnx"
inference_timeout_ms = 100
fallback_to_heuristics = true

# Model metadata
model_version = "1.0.0"
categories_supported = 15
accuracy = 0.94 # On test set
```

### Safe Search Enforcement

**Purpose**: Force safe search on search engines

**Implementation**:
- URL rewriting to add safe search parameters
- Search engine detection
- Prevent safe search bypass

**Supported Search Engines**:
```rust
const SEARCH_ENGINES: &[SearchEngine] = &[
    SearchEngine {
        domain: "google.com",
        safe_search_param: "safe=active",
        detection_pattern: r"/search\?",
    },
    SearchEngine {
        domain: "bing.com",
        safe_search_param: "adlt=strict",
        detection_pattern: r"/search\?",
    },
    SearchEngine {
        domain: "duckduckgo.com",
        safe_search_param: "kp=1",
        detection_pattern: r"/\?q=",
    },
    // ... more search engines
];
```

**Configuration**:
```toml
[web_filtering.safe_search]
enabled = true
enforce_on = ["google", "bing", "duckduckgo", "youtube"]
block_bypass_attempts = true
allow_alternative_engines = false # Block lesser-known search engines
```

### Block Page

**Purpose**: Inform users when content is blocked

**Implementation**:
- Custom HTML page served by proxy
- Age-appropriate messaging
- Request override option
- Educational information

**Block Page Content**:
```html
<!DOCTYPE html>
<html>
<head>
    <title>Content Blocked - Family Mode</title>
    <style>/* Age-appropriate styling */</style>
</head>
<body>
    <div class="container">
        <h1>This website is blocked</h1>
        <p class="reason">
            This website is blocked because it contains {{ category }} content.
        </p>

        <div class="actions">
            <button onclick="goBack()">Go Back</button>
            <button onclick="requestAccess()">Request Access</button>
        </div>

        <details class="info">
            <summary>Why is this blocked?</summary>
            <p>{{ educational_message }}</p>
        </details>

        <div class="metadata">
            <small>Blocked by DOTS Family Mode | Profile: {{ profile_name }}</small>
        </div>
    </div>
</body>
</html>
```

**Age-Appropriate Messages**:
```toml
[web_filtering.block_page.messages]
"5-7" = "This website is not for kids. Let's find something fun and safe to do!"
"8-12" = "This website isn't appropriate for your age. Ask a parent if you have questions."
"13-17" = "Access to this website is restricted. You can request permission from a parent."
```

### DNS-Based Filtering

**Purpose**: Lightweight filtering without proxy overhead

**Implementation**:
- Local DNS resolver with filtering
- Fallback when proxy unavailable
- Works with encrypted DNS (DoH, DoT)

**Configuration**:
```toml
[web_filtering.dns]
enabled = true
resolver_address = "127.0.0.1:5353"
upstream_dns = ["1.1.1.1", "8.8.8.8"]

# Blocked domains return NXDOMAIN
block_response = "nxdomain" # or "null-ip" or "block-page-ip"

# DoH/DoT support
encrypted_dns = true
doh_servers = ["https://family.cloudflare-dns.com/dns-query"]
```

**Performance**:
- Domain blocklist in memory (efficient Bloom filter)
- <1ms lookup latency
- No impact on allowed domains

## Application Content Filtering

### Content Inspection

**Purpose**: Filter inappropriate content in applications

**Approach**:
- Inspect application windows for text patterns
- OCR for image-based content (optional, expensive)
- Application-specific plugins

**Configuration**:
```toml
[content_filtering]
enabled = true
inspect_interval_seconds = 5

[content_filtering.text]
enabled = true
patterns = [
  { pattern = "(?i)suicide", action = "block", category = "self-harm" },
  { pattern = "(?i)drug.*buy", action = "alert", category = "drugs" },
]

[content_filtering.ocr]
enabled = false # Expensive, opt-in only
confidence_threshold = 0.8
```

**Implementation Details**:
- Screenshot window via Wayland protocol
- Extract text via OCR (Tesseract)
- Pattern matching against rules
- Alert or block based on severity

**Privacy Consideration**:
- Screenshots never saved to disk
- Processed in memory only
- OCR disabled by default
- Parent notification of inspection scope

### Application-Specific Filters

**Discord**:
```toml
[content_filtering.applications.discord]
enabled = true
block_direct_messages = false
block_servers = []
block_channels_with_keywords = ["nsfw", "adult"]
require_parent_approval_for_servers = true
```

**Steam**:
```toml
[content_filtering.applications.steam]
enabled = true
block_community_content = true
block_unrated_games = true
max_age_rating = "T" # E, E10+, T, M, AO
require_approval_for_purchases = true
```

**Web Browsers**:
```toml
[content_filtering.applications.firefox]
force_safe_search = true
block_private_browsing = true
block_about_config = true
disable_proxy_settings = true
```

## Terminal Content Filtering

### Command Filtering

**Purpose**: Prevent dangerous commands without breaking legitimate use

**Architecture**:
- Shell wrapper intercepts commands
- Parse and analyze before execution
- Allow/block/warn based on rules
- Educational feedback

**Configuration**:
```toml
[terminal_filtering]
enabled = true
mode = "filter" # or "monitor" or "block"

[terminal_filtering.commands]
# Always blocked
blocked = [
  "rm -rf /",
  "rm -rf /*",
  "dd if=/dev/zero of=/dev/sda",
  ":(){ :|:& };:", # fork bomb
  "chmod -R 777 /",
  "wget | bash",   # Pipe to shell
  "curl | sh",
]

# Require parent approval
approval_required = [
  "sudo *",
  "su *",
  "rm -rf *",
  "chmod +x *",
  "systemctl *",
]

# Warn but allow (educational)
warn = [
  "rm *",
  "mv * /dev/null",
  "pkill *",
]

# Always allowed
allowed = [
  "ls *",
  "cd *",
  "cat *",
  "git *",
  "cargo *",
  "python *",
]
```

**Command Classification**:
```rust
pub enum CommandRisk {
    Safe,           // No risk
    Educational,    // Good learning opportunity, warn
    Risky,          // Could cause problems, require approval
    Dangerous,      // Never allow
}

pub fn classify_command(cmd: &str) -> CommandRisk {
    // 1. Check against explicit rules
    if BLOCKED_COMMANDS.contains(cmd) {
        return CommandRisk::Dangerous;
    }

    // 2. Parse command and analyze
    let parsed = parse_shell_command(cmd)?;

    // 3. Check for dangerous patterns
    if contains_dangerous_pattern(&parsed) {
        return CommandRisk::Dangerous;
    }

    // 4. Check for risky patterns
    if contains_risky_pattern(&parsed) {
        return CommandRisk::Risky;
    }

    // 5. Check for educational opportunities
    if is_common_mistake(&parsed) {
        return CommandRisk::Educational;
    }

    CommandRisk::Safe
}
```

**Educational Warnings**:
```bash
$ rm important-file.txt
⚠️  Warning: This will permanently delete important-file.txt

    💡 Tip: Consider moving to trash instead:
        trash important-file.txt

    Or use a safer alternative:
        gio trash important-file.txt

    To proceed anyway, use: rm --force important-file.txt

Continue? [y/N]
```

**Parent Approval Flow**:
```bash
$ sudo apt install package
🔒 This command requires parent approval.

    Command: sudo apt install package
    Risk: Requires administrator privileges

    Reason: Installing system packages can affect system stability

    [Request Approval] [Cancel]

# After clicking "Request Approval"
✅ Approval request sent to parent. You'll be notified when approved.

# Parent receives notification and approves
✅ Command approved! You can run it now.
```

### Shell Integration

**Bash**:
```bash
# ~/.bashrc injection
if [[ -n "$DOTS_FAMILY_MODE" ]]; then
    # Intercept command execution
    preexec() {
        dots-terminal-filter check "$1"
        return $?
    }

    # Use Bash DEBUG trap
    trap 'preexec "$BASH_COMMAND"' DEBUG
fi
```

**Zsh**:
```zsh
# ~/.zshrc injection
if [[ -n "$DOTS_FAMILY_MODE" ]]; then
    preexec() {
        dots-terminal-filter check "$1"
        return $?
    }
fi
```

**Fish**:
```fish
# ~/.config/fish/config.fish injection
if set -q DOTS_FAMILY_MODE
    function fish_preexec --on-event fish_preexec
        dots-terminal-filter check "$argv"
        return $status
    end
end
```

**Universal PTY Wrapper** (fallback):
```rust
// Wrap PTY to intercept all input
pub struct FilteredPty {
    master: PtyMaster,
    slave: PtySlave,
    filter: Arc<CommandFilter>,
}

impl FilteredPty {
    pub fn new(command: &str) -> Result<Self> {
        let pty = openpty(None, None)?;

        // Fork process
        let pid = fork()?;

        if pid == ForkResult::Child {
            // Child: run shell
            exec_shell(&pty.slave, command)?;
        } else {
            // Parent: filter input
            let filter = CommandFilter::new();

            // Intercept write() calls to PTY
            Ok(Self {
                master: pty.master,
                slave: pty.slave,
                filter: Arc::new(filter),
            })
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        // Check command before sending to shell
        let cmd = String::from_utf8_lossy(data);

        match self.filter.check(&cmd) {
            CommandRisk::Safe => self.master.write(data),
            CommandRisk::Dangerous => {
                // Block and show error
                self.show_blocked_message(&cmd)?;
                Ok(data.len())
            }
            CommandRisk::Risky => {
                // Request approval
                self.request_approval(&cmd)?;
                Ok(data.len())
            }
            CommandRisk::Educational => {
                // Show warning, then allow
                self.show_warning(&cmd)?;
                self.master.write(data)
            }
        }
    }
}
```

### Script Detection

**Purpose**: Prevent bypassing via scripts

**Detection**:
```rust
pub fn is_script_execution(cmd: &str) -> bool {
    let patterns = [
        r"^\s*(bash|sh|zsh|fish)\s+.*\.sh",
        r"^\s*python\s+.*\.py",
        r"^\s*\./[^\s]+\.sh",
        r"^\s*source\s+",
        r"^\s*\.\s+",
    ];

    patterns.iter().any(|p| Regex::new(p).unwrap().is_match(cmd))
}
```

**Script Inspection**:
```rust
pub fn inspect_script(path: &Path) -> Result<Vec<CommandRisk>> {
    let content = fs::read_to_string(path)?;
    let lines = content.lines();

    let mut risks = Vec::new();

    for line in lines {
        // Skip comments and empty lines
        if line.trim().starts_with('#') || line.trim().is_empty() {
            continue;
        }

        // Classify each command
        let risk = classify_command(line);
        if risk != CommandRisk::Safe {
            risks.push(risk);
        }
    }

    Ok(risks)
}
```

**Behavior**:
- Script execution triggers inspection
- Highest risk level determines action
- Parent approval shows script preview
- Approved scripts cached (hash-based)

## File System Filtering

### Download Monitoring

**Purpose**: Scan downloads for threats

**Implementation**:
```toml
[content_filtering.downloads]
enabled = true
watch_directories = [
  "~/Downloads",
  "~/Desktop"
]

[content_filtering.downloads.scanning]
scan_archives = true
scan_executables = true
scan_scripts = true
max_file_size_mb = 100

[content_filtering.downloads.actions]
quarantine_path = "~/.local/share/dots-family/quarantine"
delete_threats = false # Move to quarantine instead
notify_parent = true
```

**File Scanning**:
```rust
pub async fn scan_file(path: &Path) -> Result<ScanResult> {
    // 1. Check file extension
    if is_blocked_extension(path) {
        return Ok(ScanResult::Blocked("Blocked file type"));
    }

    // 2. Check file size
    let size = fs::metadata(path)?.len();
    if size > MAX_FILE_SIZE {
        return Ok(ScanResult::TooLarge);
    }

    // 3. Scan with ClamAV (if available)
    if let Some(result) = scan_with_clamav(path).await? {
        return Ok(result);
    }

    // 4. Heuristic analysis
    if is_suspicious_content(path).await? {
        return Ok(ScanResult::Suspicious);
    }

    Ok(ScanResult::Clean)
}
```

**Integration**:
- inotify-based directory watching
- Async scanning to avoid blocking
- Quarantine suspicious files
- Parent notification and review

## Filter List Updates

### Update Mechanism

**Configuration**:
```toml
[web_filtering.updates]
enabled = true
interval_hours = 24
update_on_startup = true
update_on_metered = false
source = "https://filters.dots-family.org/lists/"

[web_filtering.updates.verification]
require_signature = true
public_key_path = "/usr/share/dots-family/signing-key.pub"
```

**Update Process**:
1. Check for new filter list versions
2. Download and verify signature
3. Test filter list (no syntax errors)
4. Atomic swap with current list
5. Log update in audit trail
6. Notify parent of changes

**Fallback**:
- If update fails, keep current lists
- Warn parent if lists >7 days stale
- Emergency update mechanism for critical threats

## Performance Optimization

### Caching

**Filter Results Cache**:
```rust
pub struct FilterCache {
    cache: Arc<RwLock<LruCache<Url, FilterResult>>>,
    ttl: Duration,
}

impl FilterCache {
    pub fn check(&self, url: &Url) -> Option<FilterResult> {
        let cache = self.cache.read().unwrap();
        cache.get(url).and_then(|(result, timestamp)| {
            if timestamp.elapsed() < self.ttl {
                Some(result.clone())
            } else {
                None
            }
        })
    }
}
```

**Configuration**:
```toml
[performance.caching]
enabled = true
cache_size = 10000 # Number of entries
ttl_seconds = 300  # 5 minutes
```

### Bloom Filters

**Purpose**: Fast negative lookups for blocklists

**Implementation**:
```rust
pub struct BlocklistBloom {
    filter: BloomFilter,
    backing_list: HashSet<String>,
}

impl BlocklistBloom {
    pub fn contains(&self, domain: &str) -> bool {
        // Fast negative check
        if !self.filter.contains(domain) {
            return false;
        }

        // Confirm with backing list (handle false positives)
        self.backing_list.contains(domain)
    }
}
```

**Benefits**:
- O(1) lookup for allowed domains
- Minimal memory overhead
- <1% false positive rate

## Privacy Considerations

### Data Minimization

- URLs logged without query parameters (configurable)
- Page content never stored
- Screenshots processed in-memory only
- Configurable retention periods

### Transparency

- Child notification of filtering scope
- Parent dashboard shows what's filtered
- Option to disable content inspection
- Clear privacy policy

### Audit Trail

All filtering actions logged:
```rust
pub struct FilterEvent {
    timestamp: DateTime<Utc>,
    profile: String,
    source: FilterSource, // Web, App, Terminal
    action: FilterAction, // Allowed, Blocked, Warned
    category: Option<ContentCategory>,
    details: Option<String>, // Sanitized
}
```

## Testing Strategy

### Unit Tests
- Filter pattern matching
- Command classification
- Category detection
- Performance benchmarks

### Integration Tests
- Proxy functionality
- End-to-end blocking
- Safe search enforcement
- Filter list updates

### Manual Testing
- Browse known safe/unsafe sites
- Test command filtering
- Verify category accuracy
- Performance testing with real usage

## Related Documentation

- PARENTAL_CONTROLS.md: Policy and restrictions
- MONITORING.md: Activity tracking integration
- RUST_APPLICATIONS.md: Filter application specs
