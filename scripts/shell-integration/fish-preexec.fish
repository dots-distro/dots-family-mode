#!/usr/bin/env fish
#
# DOTS Family Mode - Fish Shell Integration
#
# Installation:
#   1. Copy this file to ~/.config/fish/conf.d/dots-family-fish.fish
#   2. Or source manually: source /path/to/dots-family-mode/scripts/shell-integration/fish-preexec.fish

set -gx DOTS_TERMINAL_FILTER_BINARY (set -q DOTS_TERMINAL_FILTER_BINARY; and echo $DOTS_TERMINAL_FILTER_BINARY; or echo "dots-terminal-filter")
set -gx DOTS_FILTER_ENABLED (set -q DOTS_FILTER_ENABLED; and echo $DOTS_FILTER_ENABLED; or echo "true")
set -gx DOTS_FILTER_LOG (set -q DOTS_FILTER_LOG; and echo $DOTS_FILTER_LOG; or echo "/tmp/dots-terminal-filter.log")

if test "$DOTS_FILTER_ENABLED" != "true"
    exit 0
end

if not command -v $DOTS_TERMINAL_FILTER_BINARY >/dev/null 2>&1
    echo "Warning: DOTS Family Mode terminal filter not found: $DOTS_TERMINAL_FILTER_BINARY" >&2
    exit 0
end

function dots_fish_preexec --on-event fish_preexec
    set -l command_line $argv[1]
    
    if test -z (string trim "$command_line")
        return 0
    end
    
    set -l builtin_commands cd pwd echo printf read test true false \
                           alias functions type which whereis command builtin \
                           help history jobs bg fg disown kill wait \
                           set set_color export math contains count \
                           string random status abbr bind complete \
                           eval exec exit return break continue source
    
    set -l first_word (string split " " "$command_line")[1]
    
    for builtin in $builtin_commands
        if test "$first_word" = "$builtin"
            return 0
        end
    end
    
    if string match -rq '^[A-Za-z_][A-Za-z0-9_]*=' "$command_line"
        return 0
    end
    
    echo (date '+%Y-%m-%d %H:%M:%S') "[fish] Command intercepted: $command_line" >> $DOTS_FILTER_LOG 2>/dev/null
    
    if not $DOTS_TERMINAL_FILTER_BINARY --check-only --shell=fish --command="$command_line" >/dev/null 2>&1
        echo "DOTS Family Mode: Command blocked for safety" >&2
        echo "Command: $command_line" >&2
        echo "Contact your parent or guardian if you believe this is an error." >&2
        
        echo (date '+%Y-%m-%d %H:%M:%S') "[fish] Command BLOCKED: $command_line" >> $DOTS_FILTER_LOG 2>/dev/null
        
        commandline ""
        return 1
    end
    
    echo (date '+%Y-%m-%d %H:%M:%S') "[fish] Command ALLOWED: $command_line" >> $DOTS_FILTER_LOG 2>/dev/null
    return 0
end

function dots_filter_disable
    read -s -P "Enter parent password: " password
    echo
    
    if test "$password" = (set -q DOTS_PARENT_PASSWORD; and echo $DOTS_PARENT_PASSWORD; or echo "admin123")
        set -gx DOTS_FILTER_ENABLED false
        echo "Terminal filtering disabled for this session."
        echo "Use 'dots_filter_enable' to re-enable filtering."
    else
        echo "Invalid password. Filtering remains enabled."
    end
end

function dots_filter_enable
    set -gx DOTS_FILTER_ENABLED true
    echo "Terminal filtering enabled."
end

echo "DOTS Family Mode terminal filtering enabled for fish" >&2