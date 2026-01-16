# Terminal and Shell Integration

## Overview

Terminal filtering in Family Mode provides command-level safety without breaking legitimate terminal workflows. The system integrates with Ghostty terminal emulator and multiple shells (bash, zsh, fish) to intercept, analyze, and optionally block dangerous commands while providing educational feedback.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    Terminal Layer                             │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │
│  │ Ghostty  │  │  Alacritty │  │   Foot   │  │   etc.   │    │
│  └────┬─────┘  └─────┬──────┘  └─────┬────┘  └─────┬────┘    │
└───────┼──────────────┼───────────────┼─────────────┼─────────┘
        │              │               │             │
        │ PTY          │ PTY           │ PTY         │ PTY
        │              │               │             │
┌───────▼──────────────▼───────────────▼─────────────▼─────────┐
│                    Shell Layer                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                    │
│  │   Bash   │  │    Zsh   │  │   Fish   │                    │
│  │          │  │          │  │          │                    │
│  │ preexec  │  │ preexec  │  │ preexec  │                    │
│  │  hook    │  │  hook    │  │  hook    │                    │
│  └────┬─────┘  └─────┬────┘  └─────┬────┘                    │
└───────┼──────────────┼─────────────┼──────────────────────────┘
        │              │             │
        └──────────────┼─────────────┘
                       │
┌──────────────────────▼───────────────────────────────────────┐
│           dots-terminal-filter (Rust)                         │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Command Parser                              │ │
│  │  - Shell syntax parsing                                  │ │
│  │  - Argument extraction                                   │ │
│  │  - Pipe/redirect detection                               │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Risk Classifier                             │ │
│  │  - Pattern matching                                      │ │
│  │  - Heuristic analysis                                    │ │
│  │  - Educational detection                                 │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Response Generator                          │ │
│  │  - Block messages                                        │ │
│  │  - Educational warnings                                  │ │
│  │  - Approval requests                                     │ │
│  └─────────────────────────────────────────────────────────┘ │
└────────────────────────────┬───────────────────────────────────┘
                             │
                             │ DBus
                             │
┌────────────────────────────▼───────────────────────────────────┐
│              dots-family-daemon                                │
│  - Policy enforcement                                          │
│  - Parent approval                                             │
│  - Activity logging                                            │
└────────────────────────────────────────────────────────────────┘
```

## Ghostty Integration

### Native Plugin Architecture

Ghostty supports native plugins for terminal-level filtering.

**Plugin Structure**:
```
dots-ghostty-plugin/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   └── filter.rs
└── ghostty-plugin.toml
```

**ghostty-plugin.toml**:
```toml
[plugin]
name = "dots-family-filter"
version = "0.1.0"
description = "DOTS Family Mode command filtering"

[hooks]
before_execute = "filter_command"
```

**Ghostty Plugin Implementation**:
```rust
use ghostty_plugin::{Plugin, CommandContext, CommandResult};

pub struct DotsFilter {
    daemon: DaemonProxy,
}

impl Plugin for DotsFilter {
    fn new() -> Result<Self> {
        let daemon = DaemonProxy::connect()?;
        Ok(Self { daemon })
    }

    async fn before_execute(
        &self,
        ctx: &CommandContext,
    ) -> Result<CommandResult> {
        // Extract command from context
        let command = ctx.command_line();

        // Check with filter service
        let result = check_command(command).await?;

        match result {
            FilterAction::Allow => Ok(CommandResult::Proceed),
            FilterAction::Block(reason) => {
                ctx.display_message(&format!(
                    "Command blocked: {}",
                    reason
                ));
                Ok(CommandResult::Block)
            }
            FilterAction::Warn(message) => {
                ctx.display_warning(&message);
                Ok(CommandResult::Proceed)
            }
        }
    }
}

ghostty_plugin!(DotsFilter);
```

### Ghostty Configuration

**~/.config/ghostty/config**:
```toml
# Enable DOTS Family Mode plugin
plugin = "dots-family-filter"

# Plugin configuration
[plugin.dots-family-filter]
enabled = true
mode = "filter"  # monitor, filter, or block
```

## Shell Integration

### Bash Integration

**Mechanism**: Use `DEBUG` trap to intercept commands before execution.

**Integration Script** (~/.dots-family-bash.sh):
```bash
#!/bin/bash
# DOTS Family Mode - Bash Integration

# Only enable if DOTS_FAMILY_MODE is set
if [[ -n "$DOTS_FAMILY_MODE" ]]; then

    # Command interception function
    _dots_family_preexec() {
        local cmd="$BASH_COMMAND"

        # Skip if running our own filter
        if [[ "$cmd" == dots-terminal-filter* ]]; then
            return 0
        fi

        # Check command with filter
        if ! dots-terminal-filter check "$cmd"; then
            # Command was blocked
            return 1
        fi

        return 0
    }

    # Set up DEBUG trap
    trap '_dots_family_preexec' DEBUG

    # Prevent trap from being unset
    shopt -s extdebug

    # Mark as initialized
    export DOTS_FAMILY_BASH_INITIALIZED=1
fi
```

**Automatic Loading** (~/.bashrc injection):
```bash
# DOTS Family Mode
# This section is managed by dots-family-mode
# DO NOT EDIT MANUALLY

if [[ -n "$DOTS_FAMILY_MODE" ]] && [[ -z "$DOTS_FAMILY_BASH_INITIALIZED" ]]; then
    source ~/.dots-family-bash.sh
fi

# End DOTS Family Mode
```

**Installation**:
```rust
pub fn install_bash_integration() -> Result<()> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("No home dir"))?;

    // Write integration script
    let script_path = home.join(".dots-family-bash.sh");
    fs::write(&script_path, BASH_INTEGRATION_SCRIPT)?;

    // Inject into .bashrc if not already present
    let bashrc_path = home.join(".bashrc");
    let mut bashrc = fs::read_to_string(&bashrc_path)?;

    if !bashrc.contains("DOTS Family Mode") {
        bashrc.push_str("\n\n");
        bashrc.push_str(BASHRC_INJECTION);
        fs::write(&bashrc_path, bashrc)?;
    }

    Ok(())
}
```

### Zsh Integration

**Mechanism**: Use `preexec` hook.

**Integration Script** (~/.dots-family-zsh.sh):
```zsh
#!/bin/zsh
# DOTS Family Mode - Zsh Integration

if [[ -n "$DOTS_FAMILY_MODE" ]]; then

    _dots_family_preexec() {
        local cmd="$1"

        # Check command with filter
        if ! dots-terminal-filter check "$cmd"; then
            # Command was blocked
            # In zsh, we need to prevent execution
            # This is tricky - we return 1 and hope the user has err_return
            return 1
        fi

        return 0
    }

    # Register preexec hook
    autoload -Uz add-zsh-hook
    add-zsh-hook preexec _dots_family_preexec

    export DOTS_FAMILY_ZSH_INITIALIZED=1
fi
```

**Zsh-Specific Challenges**:
- `preexec` runs after command is parsed but before execution
- Cannot easily prevent execution from preexec
- Need to use `err_return` option or other mechanism

**Alternative: Zsh Line Editor Widget**:
```zsh
_dots_family_widget() {
    # Get current command line
    local cmd="$BUFFER"

    # Check with filter
    if ! dots-terminal-filter check "$cmd" 2>/dev/null; then
        # Clear buffer to prevent execution
        BUFFER=""
        zle reset-prompt
        return 1
    fi

    # Accept line if allowed
    zle accept-line
}

# Create widget
zle -N _dots_family_accept _dots_family_widget

# Bind to Enter key
bindkey '^M' _dots_family_accept
```

### Fish Integration

**Mechanism**: Use `fish_preexec` event.

**Integration Script** (~/.config/fish/conf.d/dots-family.fish):
```fish
#!/usr/bin/env fish
# DOTS Family Mode - Fish Integration

if set -q DOTS_FAMILY_MODE
    function _dots_family_preexec --on-event fish_preexec
        # Arguments: $argv[1] contains the command
        set -l cmd $argv[1]

        # Check command with filter
        if not dots-terminal-filter check $cmd
            # Command was blocked
            # In fish, returning non-zero from preexec doesn't block execution
            # We need to use a different approach

            # Option 1: Display error and hope user notices
            echo "Command blocked by Family Mode" >&2

            # Option 2: Try to cancel (doesn't always work)
            commandline -r ""
            return 1
        end

        return 0
    end

    set -g DOTS_FAMILY_FISH_INITIALIZED 1
end
```

**Fish-Specific Challenges**:
- `fish_preexec` cannot prevent execution
- Need creative solutions (clear commandline, display prominent error)
- Best approach: Use fish's built-in command wrapping

**Fish Command Wrapping** (more reliable):
```fish
# Wrap dangerous commands
for cmd in rm mv chmod sudo
    function $cmd --wraps $cmd
        # Check if command should be filtered
        set -l full_cmd "$cmd $argv"

        if not dots-terminal-filter check "$full_cmd"
            echo "Command blocked by Family Mode: $full_cmd" >&2
            return 1
        end

        # Execute original command
        command $cmd $argv
    end
end
```

## PTY-Level Filtering (Universal Fallback)

For shells without hooks or when shell integration fails, use PTY-level filtering.

### PTY Wrapper Implementation

```rust
use nix::pty::{openpty, OpenptyResult};
use nix::unistd::{fork, ForkResult};
use std::os::unix::io::{AsRawFd, RawFd};

pub struct FilteredPty {
    master: RawFd,
    child_pid: nix::unistd::Pid,
    buffer: Vec<u8>,
    command_filter: CommandFilter,
}

impl FilteredPty {
    pub fn spawn(shell: &str) -> Result<Self> {
        // Open PTY
        let OpenptyResult { master, slave } = openpty(None, None)?;

        // Fork process
        match unsafe { fork() }? {
            ForkResult::Parent { child } => {
                // Parent: return FilteredPty
                Ok(Self {
                    master: master.as_raw_fd(),
                    child_pid: child,
                    buffer: Vec::new(),
                    command_filter: CommandFilter::new()?,
                })
            }
            ForkResult::Child => {
                // Child: exec shell with slave PTY
                nix::unistd::setsid()?;
                nix::unistd::dup2(slave.as_raw_fd(), 0)?; // stdin
                nix::unistd::dup2(slave.as_raw_fd(), 1)?; // stdout
                nix::unistd::dup2(slave.as_raw_fd(), 2)?; // stderr

                std::process::Command::new(shell)
                    .exec();

                unreachable!()
            }
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<()> {
        // Accumulate data in buffer
        self.buffer.extend_from_slice(data);

        // Check if we have a complete command (ends with \n or \r)
        if data.contains(&b'\n') || data.contains(&b'\r') {
            let command = String::from_utf8_lossy(&self.buffer);

            // Filter command
            match self.command_filter.check(&command)? {
                FilterAction::Allow => {
                    // Write to PTY
                    nix::unistd::write(self.master, &self.buffer)?;
                }
                FilterAction::Block(reason) => {
                    // Display block message
                    let msg = format!("\r\nBlocked: {}\r\n", reason);
                    nix::unistd::write(self.master, msg.as_bytes())?;

                    // Write prompt again
                    nix::unistd::write(self.master, b"$ ")?;
                }
                FilterAction::Warn(warning) => {
                    // Display warning
                    let msg = format!("\r\n⚠️  {}\r\n", warning);
                    nix::unistd::write(self.master, msg.as_bytes())?;

                    // Then execute
                    nix::unistd::write(self.master, &self.buffer)?;
                }
            }

            self.buffer.clear();
        }

        Ok(())
    }
}
```

**Launch Filtered Shell**:
```bash
# User runs this instead of normal shell
dots-filtered-shell --shell bash
```

**Challenges**:
- Complex to implement correctly
- Shell features may break (job control, etc.)
- Performance overhead
- Should only be fallback

## Command Parsing and Analysis

### Command Parser

```rust
use shellwords::split;

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub executable: String,
    pub args: Vec<String>,
    pub is_sudo: bool,
    pub has_pipes: bool,
    pub has_redirects: bool,
    pub pipe_chain: Vec<String>,
    pub env_vars: HashMap<String, String>,
}

pub fn parse_command(command: &str) -> Result<ParsedCommand> {
    let trimmed = command.trim();

    // Check for sudo
    let is_sudo = trimmed.starts_with("sudo ");
    let without_sudo = if is_sudo {
        &trimmed[5..]
    } else {
        trimmed
    };

    // Check for pipes
    let has_pipes = without_sudo.contains('|');
    let pipe_chain = if has_pipes {
        without_sudo.split('|')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        vec![without_sudo.to_string()]
    };

    // Parse first command in chain
    let first_cmd = &pipe_chain[0];

    // Check for redirects
    let has_redirects = first_cmd.contains('>') || first_cmd.contains('<');

    // Split into executable and args
    let parts = split(first_cmd)?;
    let (executable, args) = if parts.is_empty() {
        (String::new(), Vec::new())
    } else {
        (parts[0].clone(), parts[1..].to_vec())
    };

    // Extract environment variables
    let env_vars = extract_env_vars(command);

    Ok(ParsedCommand {
        executable,
        args,
        is_sudo,
        has_pipes,
        has_redirects,
        pipe_chain,
        env_vars,
    })
}

fn extract_env_vars(command: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // Simple extraction: VAR=value at start of command
    let parts: Vec<&str> = command.split_whitespace().collect();

    for part in parts {
        if part.contains('=') && !part.starts_with('-') {
            let kv: Vec<&str> = part.splitn(2, '=').collect();
            if kv.len() == 2 {
                vars.insert(kv[0].to_string(), kv[1].to_string());
            }
        } else {
            // No more env vars after first non-var
            break;
        }
    }

    vars
}
```

### Risk Classification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRisk {
    Safe,
    Educational,
    Risky,
    Dangerous,
}

pub struct RiskClassifier {
    rules: Vec<FilterRule>,
    dangerous_patterns: Vec<Regex>,
}

impl RiskClassifier {
    pub fn classify(&self, parsed: &ParsedCommand) -> CommandRisk {
        // 1. Check explicit dangerous commands
        if self.is_explicitly_dangerous(parsed) {
            return CommandRisk::Dangerous;
        }

        // 2. Check dangerous patterns
        if self.matches_dangerous_pattern(parsed) {
            return CommandRisk::Dangerous;
        }

        // 3. Check if requires elevated privileges
        if parsed.is_sudo {
            return CommandRisk::Risky;
        }

        // 4. Check for destructive operations
        if self.is_destructive(parsed) {
            return CommandRisk::Risky;
        }

        // 5. Check for common mistakes
        if self.is_common_mistake(parsed) {
            return CommandRisk::Educational;
        }

        CommandRisk::Safe
    }

    fn is_explicitly_dangerous(&self, parsed: &ParsedCommand) -> bool {
        // Known dangerous commands
        matches!(parsed.executable.as_str(),
            "rm" if self.has_recursive_root(&parsed.args) |
            "dd" if self.targets_block_device(&parsed.args) |
            "mkfs" | "fdisk" | "parted" |
            "chmod" if self.chmod_dangerous(&parsed.args)
        )
    }

    fn has_recursive_root(&self, args: &[String]) -> bool {
        // Check for "rm -rf /" or similar
        let has_rf = args.contains(&"-rf".to_string()) ||
                     args.contains(&"-r".to_string()) && args.contains(&"-f".to_string());

        let has_root = args.iter().any(|a|
            a == "/" || a.starts_with("/*")
        );

        has_rf && has_root
    }

    fn is_destructive(&self, parsed: &ParsedCommand) -> bool {
        // Commands that modify/delete files
        matches!(parsed.executable.as_str(),
            "rm" | "mv" | "shred" | "wipe" |
            "truncate" | ">>" | ">"
        )
    }

    fn is_common_mistake(&self, parsed: &ParsedCommand) -> bool {
        // Patterns that suggest user error
        if parsed.executable == "rm" && !parsed.args.contains(&"-i".to_string()) {
            // rm without interactive - risky for beginners
            return true;
        }

        if parsed.executable == "mv" && !self.has_backup_option(parsed) {
            return true;
        }

        false
    }
}
```

### Educational Feedback

```rust
pub struct EducationalMessages;

impl EducationalMessages {
    pub fn get_message(risk: CommandRisk, parsed: &ParsedCommand) -> String {
        match risk {
            CommandRisk::Educational => {
                Self::educational_for_command(parsed)
            }
            CommandRisk::Risky => {
                Self::warning_for_command(parsed)
            }
            CommandRisk::Dangerous => {
                Self::block_message_for_command(parsed)
            }
            CommandRisk::Safe => String::new(),
        }
    }

    fn educational_for_command(parsed: &ParsedCommand) -> String {
        match parsed.executable.as_str() {
            "rm" => formatdoc! {"
                ⚠️  Warning: rm permanently deletes files

                💡 Tip: Use trash instead:
                    trash filename    # Move to trash (can restore)
                    rm -i filename    # Ask before deleting

                To proceed anyway, press Enter. To cancel, press Ctrl+C.
            "},
            "mv" => formatdoc! {"
                ⚠️  Warning: mv will overwrite if destination exists

                💡 Tip: Use safer alternatives:
                    mv -i source dest    # Ask before overwriting
                    mv -n source dest    # Never overwrite

                To proceed anyway, press Enter. To cancel, press Ctrl+C.
            "},
            _ => String::new(),
        }
    }

    fn warning_for_command(parsed: &ParsedCommand) -> String {
        format!(
            "⚠️  This command requires parent approval: {}\n\
             A request has been sent to your parent.\n",
            parsed.executable
        )
    }

    fn block_message_for_command(parsed: &ParsedCommand) -> String {
        format!(
            "🚫 This command is blocked for safety: {}\n\
             \n\
             This command could damage your system or delete important files.\n\
             If you believe this is a mistake, ask a parent to review the policy.\n",
            parsed.executable
        )
    }
}
```

## Script Detection and Inspection

### Script Execution Detection

```rust
pub fn is_script_execution(parsed: &ParsedCommand) -> bool {
    // Executing script file
    if matches!(parsed.executable.as_str(), "bash" | "sh" | "zsh" | "fish" | "python" | "perl" | "ruby") {
        return !parsed.args.is_empty();
    }

    // Shebang execution
    if parsed.executable.starts_with("./") || parsed.executable.starts_with("../") {
        return true;
    }

    // Source command
    if matches!(parsed.executable.as_str(), "source" | ".") {
        return true;
    }

    false
}
```

### Script Content Inspection

```rust
pub async fn inspect_script(path: &Path) -> Result<ScriptRisk> {
    let content = tokio::fs::read_to_string(path).await?;
    let lines: Vec<&str> = content.lines().collect();

    let mut max_risk = CommandRisk::Safe;
    let mut dangerous_commands = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        // Skip comments and empty lines
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Parse and classify command
        if let Ok(parsed) = parse_command(trimmed) {
            let risk = classify_command(&parsed)?;

            if risk > max_risk {
                max_risk = risk;
            }

            if matches!(risk, CommandRisk::Dangerous | CommandRisk::Risky) {
                dangerous_commands.push((i + 1, trimmed.to_string(), risk));
            }
        }
    }

    Ok(ScriptRisk {
        max_risk,
        dangerous_commands,
    })
}
```

### Script Approval Flow

```rust
pub async fn handle_script_execution(
    script_path: &Path,
    daemon: &DaemonProxy,
) -> Result<FilterAction> {
    // Inspect script
    let risk = inspect_script(script_path).await?;

    match risk.max_risk {
        CommandRisk::Safe => Ok(FilterAction::Allow),

        CommandRisk::Educational => {
            let message = format!(
                "This script contains {} potentially risky commands.\n\
                 Review carefully before running.",
                risk.dangerous_commands.len()
            );
            Ok(FilterAction::Warn(message))
        }

        CommandRisk::Risky | CommandRisk::Dangerous => {
            // Request parent approval
            let approval = daemon.request_script_approval(
                script_path,
                &risk.dangerous_commands,
            ).await?;

            if approval {
                Ok(FilterAction::Allow)
            } else {
                Ok(FilterAction::Block(
                    "Script contains dangerous commands".to_string()
                ))
            }
        }
    }
}
```

## Testing

### Unit Tests

```rust
#[test]
fn test_command_parsing() {
    let cmd = "sudo rm -rf /tmp/test";
    let parsed = parse_command(cmd).unwrap();

    assert_eq!(parsed.executable, "rm");
    assert_eq!(parsed.args, vec!["-rf", "/tmp/test"]);
    assert!(parsed.is_sudo);
}

#[test]
fn test_dangerous_command_detection() {
    let classifier = RiskClassifier::new();

    let cmd = parse_command("rm -rf /").unwrap();
    assert_eq!(classifier.classify(&cmd), CommandRisk::Dangerous);

    let cmd = parse_command("ls -la").unwrap();
    assert_eq!(classifier.classify(&cmd), CommandRisk::Safe);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_bash_integration() {
    // Set up test environment
    std::env::set_var("DOTS_FAMILY_MODE", "1");

    // Run bash with integration
    let output = Command::new("bash")
        .arg("-c")
        .arg("source ~/.dots-family-bash.sh && rm -rf /")
        .output()
        .await?;

    // Should be blocked
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("blocked"));
}
```

## Performance Considerations

- Command parsing: <1ms
- Risk classification: <1ms
- DBus communication: 1-5ms
- Total overhead: <10ms per command (acceptable)

## Related Documentation

- CONTENT_FILTERING.md: Overall filtering architecture
- RUST_APPLICATIONS.md: Terminal filter application details
- PARENTAL_CONTROLS.md: Terminal filtering policies
