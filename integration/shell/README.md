# DOTS Family Mode Shell Integration

This directory contains shell integration scripts that enable command filtering for bash, zsh, and fish shells.

## Quick Start

```bash
# Install for all shells (current user)
./install.sh

# Install for specific shell
./install.sh --shell bash

# Install system-wide (requires root)
sudo ./install.sh --system

# Uninstall
./install.sh --uninstall
```

## Files

- `dots-bash-integration.sh` - Bash shell integration
- `dots-zsh-integration.sh` - Zsh shell integration  
- `dots-fish-integration.fish` - Fish shell integration
- `install.sh` - Automated installer script

## Features

### Command Filtering
- Pre-execution command analysis
- Risk-based blocking and warnings
- Educational feedback for blocked commands
- Parent approval system integration

### Multi-Shell Support
- **Bash**: Uses preexec hooks (requires bash-preexec) or DEBUG trap fallback
- **Zsh**: Native preexec support
- **Fish**: Uses fish_preexec event system

### User Controls
- Enable/disable filtering per session
- Status checking
- Manual setup and teardown
- Configuration via environment variables

## Installation Methods

### Automatic Installation
```bash
# All shells, current user
./install.sh

# Specific shell only
./install.sh --shell zsh

# System-wide installation
sudo ./install.sh --system
```

### Manual Installation

#### Bash
Add to `~/.bashrc`:
```bash
if [ -f "/path/to/dots-bash-integration.sh" ]; then
    source "/path/to/dots-bash-integration.sh"
    dots_filter_setup
fi
```

#### Zsh
Add to `~/.zshrc`:
```zsh
if [[ -f "/path/to/dots-zsh-integration.sh" ]]; then
    source "/path/to/dots-zsh-integration.sh" 
    dots_filter_setup
fi
```

#### Fish
Add to `~/.config/fish/config.fish`:
```fish
if test -f "/path/to/dots-fish-integration.fish"
    source "/path/to/dots-fish-integration.fish"
    dots_filter_setup
end
```

## Configuration

Environment variables:
- `DOTS_FILTER_ENABLED` - Enable/disable filtering (default: true)
- `DOTS_FILTER_BIN` - Path to filter binary (default: dots-terminal-filter)
- `DOTS_FILTER_MODE` - Filter mode (default: check-only)
- `DOTS_FILTER_AUTO_SETUP` - Auto-enable on shell start (default: false)

## Usage

### Available Commands
- `dots_filter_setup` - Enable filtering for current session
- `dots_filter_disable` - Disable filtering for current session
- `dots_filter_status` - Show current filter status
- `dots_filter_help` - Show help information

### Example Session
```bash
$ dots_filter_status
DOTS Family Mode Terminal Filter Status:
  Enabled: true
  Binary: dots-terminal-filter
  Available: yes
  Active: true
  Method: preexec hook

$ sudo rm -rf /
DOTS Family Mode: Command blocked for safety
Command: sudo rm -rf /

$ dots_filter_disable
DOTS Family Mode terminal filtering disabled

$ dots_filter_setup
DOTS Family Mode terminal filtering enabled
```

## Dependencies

### Required
- `dots-terminal-filter` binary in PATH
- Modern shell (bash 4.0+, zsh 5.0+, fish 3.0+)

### Optional
- `bash-preexec` for enhanced bash support
- System DBus for parent approval features

## Security Considerations

- Integration scripts run in user context
- No privilege escalation required for basic operation
- Parent approval requires daemon connection
- Filter bypass protection through multiple hook methods

## Troubleshooting

### Filter Not Working
1. Check `dots_filter_status` output
2. Verify `dots-terminal-filter` is in PATH
3. Restart shell after installation
4. Check shell-specific requirements

### Bash Issues
- Install bash-preexec for full functionality
- DEBUG trap method has limitations
- Check bash version (4.0+ required)

### Permission Errors
- Use `--user` flag for user-only install
- System install requires root privileges
- Check file permissions on integration scripts

### Performance Impact
- Minimal overhead per command
- Filter runs in separate process
- Caching optimizes repeated commands

## Uninstalling

```bash
# Remove integration
./install.sh --uninstall

# Manual removal
# Remove integration lines from shell RC files
# Delete integration files from install directory
```

## Development

### Testing Integration
```bash
# Dry run installation
./install.sh --dry-run

# Test specific shell
./install.sh --shell bash --dry-run

# Manual testing
source dots-bash-integration.sh
dots_filter_setup
```

### Adding New Shells
1. Create new integration script
2. Follow existing patterns for preexec hooks
3. Update installer script
4. Add tests and documentation