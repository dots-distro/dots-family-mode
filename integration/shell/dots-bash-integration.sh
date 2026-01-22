#!/bin/bash
# DOTS Family Mode - Bash Integration
# This script provides command filtering for bash shells

# Configuration
DOTS_FILTER_ENABLED="${DOTS_FILTER_ENABLED:-true}"
DOTS_FILTER_BIN="${DOTS_FILTER_BIN:-dots-terminal-filter}"
DOTS_FILTER_MODE="${DOTS_FILTER_MODE:-check-only}"

# Internal state
_dots_filter_active=false

# Check if DOTS filtering is available
_dots_check_available() {
    if [[ "$DOTS_FILTER_ENABLED" != "true" ]]; then
        return 1
    fi
    
    if ! command -v "$DOTS_FILTER_BIN" >/dev/null 2>&1; then
        return 1
    fi
    
    return 0
}

# Filter a command before execution
_dots_filter_command() {
    local cmd="$1"
    
    if [[ -z "$cmd" ]]; then
        return 0
    fi
    
    # Skip filtering for certain commands
    case "$cmd" in
        "exit" | "help" | "history" | "jobs" | "bg" | "fg" | "cd "*| "pwd" | "echo "*| "printf "*)
            return 0
            ;;
        "_dots_"*)
            return 0
            ;;
    esac
    
    # Run the filter
    if "$DOTS_FILTER_BIN" --check-only --command "$cmd" --shell bash 2>/dev/null; then
        return 0
    else
        local exit_code=$?
        if [[ $exit_code -eq 1 ]]; then
            echo "DOTS Family Mode: Command blocked for safety" >&2
            echo "Command: $cmd" >&2
            echo "Type 'help' for more information" >&2
            return 1
        fi
        return 0
    fi
}

# Preexec hook for bash (requires bash-preexec)
_dots_preexec() {
    if [[ "$_dots_filter_active" == "true" ]]; then
        return 0
    fi
    
    if ! _dots_check_available; then
        return 0
    fi
    
    _dots_filter_active=true
    local cmd="$1"
    
    if ! _dots_filter_command "$cmd"; then
        # Command was blocked - prevent execution
        _dots_filter_active=false
        return 1
    fi
    
    _dots_filter_active=false
    return 0
}

# Alternative method using DEBUG trap (fallback if bash-preexec not available)
_dots_debug_trap() {
    if [[ "$BASH_COMMAND" == "_dots_"* ]] || [[ "$_dots_filter_active" == "true" ]]; then
        return 0
    fi
    
    if ! _dots_check_available; then
        return 0
    fi
    
    # Only filter interactive commands
    if [[ $- =~ i ]]; then
        _dots_filter_active=true
        local cmd="$BASH_COMMAND"
        
        if ! _dots_filter_command "$cmd"; then
            _dots_filter_active=false
            echo "Command execution blocked" >&2
        fi
        _dots_filter_active=false
    fi
}

# Setup function
dots_filter_setup() {
    if ! _dots_check_available; then
        echo "DOTS Family Mode terminal filter not available" >&2
        return 1
    fi
    
    # Try to use bash-preexec if available
    if declare -f preexec_functions >/dev/null 2>&1; then
        echo "Setting up DOTS filter with bash-preexec"
        preexec_functions+=(_dots_preexec)
    else
        echo "Setting up DOTS filter with DEBUG trap (limited functionality)"
        echo "Consider installing bash-preexec for full command filtering"
        trap '_dots_debug_trap' DEBUG
    fi
    
    echo "DOTS Family Mode terminal filtering enabled"
    echo "Filter binary: $DOTS_FILTER_BIN"
    echo "Use 'dots_filter_disable' to disable filtering"
}

# Disable function
dots_filter_disable() {
    if declare -f preexec_functions >/dev/null 2>&1; then
        local new_functions=()
        for func in "${preexec_functions[@]}"; do
            if [[ "$func" != "_dots_preexec" ]]; then
                new_functions+=("$func")
            fi
        done
        preexec_functions=("${new_functions[@]}")
    else
        trap - DEBUG
    fi
    
    echo "DOTS Family Mode terminal filtering disabled"
}

# Status function
dots_filter_status() {
    echo "DOTS Family Mode Terminal Filter Status:"
    echo "  Enabled: $DOTS_FILTER_ENABLED"
    echo "  Binary: $DOTS_FILTER_BIN"
    echo "  Available: $(_dots_check_available && echo "yes" || echo "no")"
    
    if declare -f preexec_functions >/dev/null 2>&1; then
        local has_hook=false
        for func in "${preexec_functions[@]}"; do
            if [[ "$func" == "_dots_preexec" ]]; then
                has_hook=true
                break
            fi
        done
        echo "  Active: $has_hook"
        echo "  Method: bash-preexec"
    else
        local trap_cmd=$(trap -p DEBUG)
        if [[ "$trap_cmd" == *"_dots_debug_trap"* ]]; then
            echo "  Active: true"
        else
            echo "  Active: false"
        fi
        echo "  Method: DEBUG trap"
    fi
}

# Help function
dots_filter_help() {
    cat <<EOF
DOTS Family Mode Terminal Filter - Bash Integration

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
  Add to ~/.bashrc:
    source /path/to/dots-bash-integration.sh
    dots_filter_setup

Dependencies:
  - dots-terminal-filter binary in PATH
  - bash-preexec (recommended): https://github.com/rcaloras/bash-preexec

For help with blocked commands, contact your parent or guardian.
EOF
}

# Auto-setup if requested
if [[ "${DOTS_FILTER_AUTO_SETUP:-}" == "true" ]] && _dots_check_available; then
    dots_filter_setup
fi