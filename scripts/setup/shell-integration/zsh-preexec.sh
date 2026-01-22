#!/bin/zsh
#
# DOTS Family Mode - Zsh Shell Integration
#
# Installation:
#   1. Source this script in your ~/.zshrc:
#      source /path/to/dots-family-mode/scripts/shell-integration/zsh-preexec.sh
#   2. Or copy to /etc/zsh/zshrc.d/dots-family-zsh.sh for system-wide installation

DOTS_TERMINAL_FILTER_BINARY="${DOTS_TERMINAL_FILTER_BINARY:-dots-terminal-filter}"
DOTS_FILTER_ENABLED="${DOTS_FILTER_ENABLED:-true}"
DOTS_FILTER_LOG="${DOTS_FILTER_LOG:-/tmp/dots-terminal-filter.log}"

if [[ "${DOTS_FILTER_ENABLED}" != "true" ]]; then
    return 0
fi

if ! command -v "${DOTS_TERMINAL_FILTER_BINARY}" >/dev/null 2>&1; then
    echo "Warning: DOTS Family Mode terminal filter not found: ${DOTS_TERMINAL_FILTER_BINARY}" >&2
    return 0
fi

dots_zsh_preexec() {
    local command="$1"
    
    if [[ -z "${command// }" ]]; then
        return 0
    fi
    
    local builtin_commands=(
        "cd" "pwd" "echo" "printf" "read" "test" "[" "[[" "true" "false"
        "alias" "unalias" "type" "which" "whereis" "command" "builtin"
        "help" "history" "jobs" "bg" "fg" "disown" "kill" "wait" "trap"
        "set" "unset" "export" "readonly" "local" "declare" "typeset"
        "setopt" "unsetopt" "bindkey" "vared" "zle" "autoload" "compinit"
        "rehash" "hash" "unhash" "whence" "where" "functions" "disable"
    )
    
    local first_word="${command%% *}"
    
    for builtin in "${builtin_commands[@]}"; do
        if [[ "${first_word}" == "${builtin}" ]]; then
            return 0
        fi
    done
    
    if [[ "${command}" =~ ^[A-Za-z_][A-Za-z0-9_]*= ]]; then
        return 0
    fi
    
    echo "$(date '+%Y-%m-%d %H:%M:%S') [zsh] Command intercepted: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
    
    if ! "${DOTS_TERMINAL_FILTER_BINARY}" --check-only --shell=zsh --command="${command}" >/dev/null 2>&1; then
        echo "DOTS Family Mode: Command blocked for safety" >&2
        echo "Command: ${command}" >&2
        echo "Contact your parent or guardian if you believe this is an error." >&2
        
        echo "$(date '+%Y-%m-%d %H:%M:%S') [zsh] Command BLOCKED: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
        
        return 1
    fi
    
    echo "$(date '+%Y-%m-%d %H:%M:%S') [zsh] Command ALLOWED: ${command}" >> "${DOTS_FILTER_LOG}" 2>/dev/null
    return 0
}

dots_setup_zsh_preexec() {
    if [[ "${DOTS_ZSH_PREEXEC_INSTALLED}" == "true" ]]; then
        return 0
    fi
    
    autoload -Uz add-zsh-hook
    add-zsh-hook preexec dots_zsh_preexec
    
    export DOTS_ZSH_PREEXEC_INSTALLED="true"
    echo "DOTS Family Mode terminal filtering enabled for zsh" >&2
}

dots_setup_zsh_preexec

dots_filter_disable() {
    local password
    read -s "password?Enter parent password: "
    echo
    
    if [[ "${password}" == "${DOTS_PARENT_PASSWORD:-admin123}" ]]; then
        export DOTS_FILTER_ENABLED="false"
        echo "Terminal filtering disabled for this session."
        echo "Use 'dots_filter_enable' to re-enable filtering."
    else
        echo "Invalid password. Filtering remains enabled."
    fi
    
    unset password
}

dots_filter_enable() {
    export DOTS_FILTER_ENABLED="true"
    echo "Terminal filtering enabled."
}