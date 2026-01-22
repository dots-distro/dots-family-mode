#!/bin/zsh
# DOTS Family Mode - Zsh Integration
# This script provides command filtering for zsh shells

# Configuration
DOTS_FILTER_ENABLED="${DOTS_FILTER_ENABLED:-true}"
DOTS_FILTER_BIN="${DOTS_FILTER_BIN:-dots-terminal-filter}"
DOTS_FILTER_MODE="${DOTS_FILTER_MODE:-check-only}"

# Internal state
typeset -g _dots_filter_active=false

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
        "exit" | "help" | "history" | "jobs" | "bg" | "fg" | cd\ * | "pwd" | echo\ * | printf\ *)
            return 0
            ;;
        "_dots_"*)
            return 0
            ;;
    esac
    
    # Run the filter
    if "$DOTS_FILTER_BIN" --check-only --command "$cmd" --shell zsh 2>/dev/null; then
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

# Preexec hook for zsh
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
        _dots_filter_active=false
        print -P "%F{red}Command blocked by DOTS Family Mode%f" >&2
        return 1
    fi
    
    _dots_filter_active=false
    return 0
}

# Setup function
dots_filter_setup() {
    if ! _dots_check_available; then
        echo "DOTS Family Mode terminal filter not available" >&2
        return 1
    fi
    
    # Add to preexec_functions array
    if ! (( ${+preexec_functions} )); then
        typeset -ga preexec_functions
    fi
    
    # Check if already installed
    if (( ${preexec_functions[(I)_dots_preexec]} )); then
        echo "DOTS filter already active"
        return 0
    fi
    
    preexec_functions+=(_dots_preexec)
    
    echo "DOTS Family Mode terminal filtering enabled"
    echo "Filter binary: $DOTS_FILTER_BIN"
    echo "Use 'dots_filter_disable' to disable filtering"
}

# Disable function
dots_filter_disable() {
    if (( ${+preexec_functions} )); then
        preexec_functions=(${preexec_functions:#_dots_preexec})
    fi
    
    echo "DOTS Family Mode terminal filtering disabled"
}

# Status function
dots_filter_status() {
    echo "DOTS Family Mode Terminal Filter Status:"
    echo "  Enabled: $DOTS_FILTER_ENABLED"
    echo "  Binary: $DOTS_FILTER_BIN"
    echo "  Available: $(_dots_check_available && echo "yes" || echo "no")"
    
    if (( ${+preexec_functions} )) && (( ${preexec_functions[(I)_dots_preexec]} )); then
        echo "  Active: true"
    else
        echo "  Active: false"
    fi
    echo "  Method: preexec hook"
}

# Help function
dots_filter_help() {
    cat <<EOF
DOTS Family Mode Terminal Filter - Zsh Integration

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
  Add to ~/.zshrc:
    source /path/to/dots-zsh-integration.sh
    dots_filter_setup

Dependencies:
  - dots-terminal-filter binary in PATH
  - zsh with preexec support

For help with blocked commands, contact your parent or guardian.
EOF
}

# Auto-setup if requested
if [[ "${DOTS_FILTER_AUTO_SETUP:-}" == "true" ]] && _dots_check_available; then
    dots_filter_setup
fi