#!/bin/bash
#
# DOTS Family Mode - Bash Shell Integration
#
# This script provides preexec functionality for bash to intercept commands
# before execution and check them against the DOTS Family Mode terminal filter.
#
# Installation:
#   1. Source this script in your ~/.bashrc:
#      source /path/to/dots-family-mode/scripts/shell-integration/bash-preexec.sh
#   2. Or copy to /etc/profile.d/dots-family-bash.sh for system-wide installation
#
# The script uses the preexec mechanism to intercept commands before execution.

# Configuration
DOTS_TERMINAL_FILTER_BINARY="${DOTS_TERMINAL_FILTER_BINARY:-dots-terminal-filter}"
DOTS_FILTER_ENABLED="${DOTS_FILTER_ENABLED:-true}"
DOTS_FILTER_LOG="${DOTS_FILTER_LOG:-/tmp/dots-terminal-filter.log}"

# Check if we're in a DOTS-controlled environment
if [[ "${DOTS_FILTER_ENABLED}" != "true" ]]; then
    return 0
fi

# Check if the filter binary exists
if ! command -v "${DOTS_TERMINAL_FILTER_BINARY}" >/dev/null 2>&1; then
    echo "Warning: DOTS Family Mode terminal filter not found: ${DOTS_TERMINAL_FILTER_BINARY}" >&2
    return 0
fi

# Function to be called before each command execution
dots_preexec() {
    local command="$1"
    
    # Skip empty commands
    if [[ -z "${command// }" ]]; then
        return 0
    fi
    
    # Skip built-in commands that don't need filtering
    local builtin_commands=(
        "cd" "pwd" "echo" "printf" "read" "test" "[" "[[" "true" "false"
        ":" "." "source" "eval" "exec" "exit" "return" "break" "continue"
        "alias" "unalias" "type" "which" "whereis" "command" "builtin"
        "help" "history" "jobs" "bg" "fg" "disown" "kill" "wait" "trap"
        "set" "unset" "export" "readonly" "local" "declare" "typeset"
        "if" "then" "else" "elif" "fi" "case" "esac" "for" "while" "until"
        "do" "done" "function" "select" "time" "coproc" "{" "}" "(" ")"
    )
    
    local first_word
    first_word=$(echo "${command}" | awk '{print $1}')
    
    # Check if it's a builtin command
    for builtin in "${builtin_commands[@]}"; do
        if [[ "${first_word}" == "${builtin}" ]]; then
            return 0
        fi
    done
    
    # Check if it's a variable assignment (no filtering needed)
    if [[ "${command}" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
        return 0
    fi
    
    # Log the command attempt
    echo "$(date '+%Y-%m-%d %H:%M:%S') [bash] Command intercepted: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
    
    # Call the DOTS terminal filter to evaluate the command
    if ! "${DOTS_TERMINAL_FILTER_BINARY}" --check-only --shell=bash --command="${command}" >/dev/null 2>&1; then
        echo "DOTS Family Mode: Command blocked for safety" >&2
        echo "Command: ${command}" >&2
        echo "Contact your parent or guardian if you believe this is an error." >&2
        
        # Log the blocked command
        echo "$(date '+%Y-%m-%d %H:%M:%S') [bash] Command BLOCKED: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
        
        # Prevent command execution by setting a trap
        trap 'echo "Command execution prevented by DOTS Family Mode" >&2; trap - DEBUG; return 130' DEBUG
        return 1
    fi
    
    # Log the allowed command
    echo "$(date '+%Y-%m-%d %H:%M:%S') [bash] Command ALLOWED: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
    return 0
}

# Install the preexec hook using DEBUG trap
dots_setup_preexec() {
    # Only set up if not already done
    if [[ "${DOTS_PREEXEC_INSTALLED}" == "true" ]]; then
        return 0
    fi
    
    # Function to handle the DEBUG trap
    dots_debug_trap() {
        local command="${BASH_COMMAND}"
        
        # Skip if we're in the middle of setting up the trap
        if [[ "${command}" =~ dots_debug_trap|dots_preexec|DOTS_ ]]; then
            return 0
        fi
        
        # Call our preexec function
        dots_preexec "${command}"
    }
    
    # Set the DEBUG trap
    trap 'dots_debug_trap' DEBUG
    
    # Mark as installed
    export DOTS_PREEXEC_INSTALLED="true"
    
    echo "DOTS Family Mode terminal filtering enabled for bash" >&2
}

# Install the preexec mechanism
dots_setup_preexec

# Cleanup function for when shell exits
dots_cleanup() {
    trap - DEBUG
    unset DOTS_PREEXEC_INSTALLED
}

# Set up cleanup on shell exit
trap 'dots_cleanup' EXIT

# Function to temporarily disable filtering (for parent override)
dots_filter_disable() {
    local password
    read -s -p "Enter parent password: " password
    echo
    
    # In production, this should validate against the stored parent password
    # For now, we'll use a simple placeholder
    if [[ "${password}" == "${DOTS_PARENT_PASSWORD:-admin123}" ]]; then
        export DOTS_FILTER_ENABLED="false"
        echo "Terminal filtering disabled for this session."
        echo "Use 'dots_filter_enable' to re-enable filtering."
    else
        echo "Invalid password. Filtering remains enabled."
    fi
    
    unset password
}

# Function to re-enable filtering
dots_filter_enable() {
    export DOTS_FILTER_ENABLED="true"
    echo "Terminal filtering enabled."
}

# Make functions available to user
export -f dots_filter_disable
export -f dots_filter_enable

# Completion functions for common commands
complete -c dots_filter_disable
complete -c dots_filter_enable