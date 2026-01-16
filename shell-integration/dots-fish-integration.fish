# DOTS Family Mode - Fish Integration
# This script provides command filtering for fish shells

# Configuration
set -q DOTS_FILTER_ENABLED; or set -gx DOTS_FILTER_ENABLED true
set -q DOTS_FILTER_BIN; or set -gx DOTS_FILTER_BIN dots-terminal-filter
set -q DOTS_FILTER_MODE; or set -gx DOTS_FILTER_MODE check-only

# Internal state
set -g _dots_filter_active false

# Check if DOTS filtering is available
function _dots_check_available
    if test "$DOTS_FILTER_ENABLED" != true
        return 1
    end
    
    if not command -q "$DOTS_FILTER_BIN"
        return 1
    end
    
    return 0
end

# Filter a command before execution
function _dots_filter_command
    set -l cmd "$argv[1]"
    
    if test -z "$cmd"
        return 0
    end
    
    # Skip filtering for certain commands
    switch "$cmd"
        case "exit" "help" "history" "jobs" "bg" "fg" "cd *" "pwd" "echo *" "printf *"
            return 0
        case "_dots_*"
            return 0
    end
    
    # Run the filter
    if $DOTS_FILTER_BIN --check-only --command "$cmd" --shell fish 2>/dev/null
        return 0
    else
        set -l exit_code $status
        if test $exit_code -eq 1
            echo "DOTS Family Mode: Command blocked for safety" >&2
            echo "Command: $cmd" >&2
            echo "Type 'help' for more information" >&2
            return 1
        end
        return 0
    end
end

# Preexec hook for fish
function _dots_preexec --on-event fish_preexec
    if test "$_dots_filter_active" = true
        return 0
    end
    
    if not _dots_check_available
        return 0
    end
    
    set -g _dots_filter_active true
    set -l cmd "$argv[1]"
    
    if not _dots_filter_command "$cmd"
        set -g _dots_filter_active false
        set_color red
        echo "Command blocked by DOTS Family Mode" >&2
        set_color normal
        return 1
    end
    
    set -g _dots_filter_active false
    return 0
end

# Setup function
function dots_filter_setup
    if not _dots_check_available
        echo "DOTS Family Mode terminal filter not available" >&2
        return 1
    end
    
    # Enable the preexec hook
    functions -e _dots_preexec 2>/dev/null
    function _dots_preexec --on-event fish_preexec
        if test "$_dots_filter_active" = true
            return 0
        end
        
        if not _dots_check_available
            return 0
        end
        
        set -g _dots_filter_active true
        set -l cmd "$argv[1]"
        
        if not _dots_filter_command "$cmd"
            set -g _dots_filter_active false
            set_color red
            echo "Command blocked by DOTS Family Mode" >&2
            set_color normal
            return 1
        end
        
        set -g _dots_filter_active false
        return 0
    end
    
    echo "DOTS Family Mode terminal filtering enabled"
    echo "Filter binary: $DOTS_FILTER_BIN"
    echo "Use 'dots_filter_disable' to disable filtering"
end

# Disable function
function dots_filter_disable
    functions -e _dots_preexec 2>/dev/null
    
    echo "DOTS Family Mode terminal filtering disabled"
end

# Status function
function dots_filter_status
    echo "DOTS Family Mode Terminal Filter Status:"
    echo "  Enabled: $DOTS_FILTER_ENABLED"
    echo "  Binary: $DOTS_FILTER_BIN"
    
    if _dots_check_available
        echo "  Available: yes"
    else
        echo "  Available: no"
    end
    
    if functions -q _dots_preexec
        echo "  Active: true"
    else
        echo "  Active: false"
    end
    echo "  Method: fish_preexec event"
end

# Help function
function dots_filter_help
    echo "DOTS Family Mode Terminal Filter - Fish Integration

Commands:
  dots_filter_setup    - Enable terminal command filtering
  dots_filter_disable  - Disable terminal command filtering  
  dots_filter_status   - Show current filter status
  dots_filter_help     - Show this help message

Environment Variables:
  DOTS_FILTER_ENABLED  - Enable/disable filtering (true/false)
  DOTS_FILTER_BIN      - Path to filter binary (default: dots-terminal-filter)
  DOTS_FILTER_MODE     - Filter mode (default: check-only)

Installation:
  Add to ~/.config/fish/config.fish:
    source /path/to/dots-fish-integration.fish
    dots_filter_setup

Dependencies:
  - dots-terminal-filter binary in PATH
  - fish with event support

For help with blocked commands, contact your parent or guardian."
end

# Auto-setup if requested
if test "$DOTS_FILTER_AUTO_SETUP" = true; and _dots_check_available
    dots_filter_setup
end