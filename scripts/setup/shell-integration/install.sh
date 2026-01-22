#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
SYSTEM_SHELL_DIR="/etc/profile.d"
USER_SHELL_DIR="${HOME}/.config/dots"

show_help() {
    cat <<EOF
DOTS Family Mode Terminal Filter - Shell Integration Install Script

Usage: $0 [OPTIONS]

OPTIONS:
    --user          Install for current user only (default)
    --system        Install system-wide (requires root)
    --shell SHELL   Install for specific shell (bash, zsh, fish, all)
    --uninstall     Remove shell integration
    --help          Show this help message

EXAMPLES:
    $0 --user --shell bash
    sudo $0 --system --shell all
    $0 --uninstall

The script will:
1. Copy shell integration scripts to appropriate locations
2. Set up shell hooks for command filtering
3. Configure the DOTS terminal filter service
EOF
}

install_bash_integration() {
    local target_dir="$1"
    local script_name="dots-family-bash.sh"
    
    echo "Installing bash integration..."
    
    if [[ "$target_dir" == "$SYSTEM_SHELL_DIR" ]]; then
        cp "${SCRIPT_DIR}/bash-preexec.sh" "${target_dir}/${script_name}"
        chmod 644 "${target_dir}/${script_name}"
        echo "Bash integration installed to ${target_dir}/${script_name}"
        echo "All bash sessions will now use DOTS filtering."
    else
        mkdir -p "$target_dir"
        cp "${SCRIPT_DIR}/bash-preexec.sh" "${target_dir}/${script_name}"
        
        local bashrc="${HOME}/.bashrc"
        local source_line="source \"${target_dir}/${script_name}\""
        
        if ! grep -q "$source_line" "$bashrc" 2>/dev/null; then
            echo "" >> "$bashrc"
            echo "# DOTS Family Mode Terminal Filter" >> "$bashrc"
            echo "$source_line" >> "$bashrc"
            echo "Bash integration installed. Added source line to $bashrc"
        else
            echo "Bash integration already configured in $bashrc"
        fi
    fi
}

install_zsh_integration() {
    local target_dir="$1"
    
    echo "Installing zsh integration..."
    
    if [[ "$target_dir" == "$SYSTEM_SHELL_DIR" ]]; then
        local zsh_system_dir="/etc/zsh/zshrc.d"
        mkdir -p "$zsh_system_dir"
        cp "${SCRIPT_DIR}/zsh-preexec.sh" "${zsh_system_dir}/dots-family-zsh.sh"
        chmod 644 "${zsh_system_dir}/dots-family-zsh.sh"
        echo "Zsh integration installed to ${zsh_system_dir}/dots-family-zsh.sh"
    else
        mkdir -p "$target_dir"
        cp "${SCRIPT_DIR}/zsh-preexec.sh" "${target_dir}/dots-family-zsh.sh"
        
        local zshrc="${HOME}/.zshrc"
        local source_line="source \"${target_dir}/dots-family-zsh.sh\""
        
        if ! grep -q "$source_line" "$zshrc" 2>/dev/null; then
            echo "" >> "$zshrc"
            echo "# DOTS Family Mode Terminal Filter" >> "$zshrc"
            echo "$source_line" >> "$zshrc"
            echo "Zsh integration installed. Added source line to $zshrc"
        else
            echo "Zsh integration already configured in $zshrc"
        fi
    fi
}

install_fish_integration() {
    local target_dir="$1"
    
    echo "Installing fish integration..."
    
    if [[ "$target_dir" == "$SYSTEM_SHELL_DIR" ]]; then
        local fish_system_dir="/etc/fish/conf.d"
        mkdir -p "$fish_system_dir"
        cp "${SCRIPT_DIR}/fish-preexec.fish" "${fish_system_dir}/dots-family-fish.fish"
        chmod 644 "${fish_system_dir}/dots-family-fish.fish"
        echo "Fish integration installed to ${fish_system_dir}/dots-family-fish.fish"
    else
        local fish_user_dir="${HOME}/.config/fish/conf.d"
        mkdir -p "$fish_user_dir"
        cp "${SCRIPT_DIR}/fish-preexec.fish" "${fish_user_dir}/dots-family-fish.fish"
        echo "Fish integration installed to ${fish_user_dir}/dots-family-fish.fish"
    fi
}

uninstall_integration() {
    echo "Uninstalling DOTS shell integration..."
    
    local files_to_remove=(
        "/etc/profile.d/dots-family-bash.sh"
        "/etc/zsh/zshrc.d/dots-family-zsh.sh"
        "/etc/fish/conf.d/dots-family-fish.fish"
        "${HOME}/.config/dots/dots-family-bash.sh"
        "${HOME}/.config/dots/dots-family-zsh.sh" 
        "${HOME}/.config/fish/conf.d/dots-family-fish.fish"
    )
    
    for file in "${files_to_remove[@]}"; do
        if [[ -f "$file" ]]; then
            rm -f "$file"
            echo "Removed $file"
        fi
    done
    
    local bashrc="${HOME}/.bashrc"
    local zshrc="${HOME}/.zshrc"
    
    if [[ -f "$bashrc" ]]; then
        sed -i '/# DOTS Family Mode Terminal Filter/,+1d' "$bashrc" 2>/dev/null || true
        echo "Cleaned up $bashrc"
    fi
    
    if [[ -f "$zshrc" ]]; then
        sed -i '/# DOTS Family Mode Terminal Filter/,+1d' "$zshrc" 2>/dev/null || true  
        echo "Cleaned up $zshrc"
    fi
    
    echo "Shell integration uninstalled."
}

install_binary() {
    echo "Installing DOTS terminal filter binary..."
    
    local binary_src="${SCRIPT_DIR}/../target/x86_64-unknown-linux-gnu/release/dots-terminal-filter"
    
    if [[ ! -f "$binary_src" ]]; then
        echo "Building terminal filter binary..."
        (cd "${SCRIPT_DIR}/.." && cargo build --release -p dots-terminal-filter)
    fi
    
    if [[ ! -f "$binary_src" ]]; then
        echo "Error: Could not find or build terminal filter binary" >&2
        exit 1
    fi
    
    cp "$binary_src" "${INSTALL_DIR}/dots-terminal-filter"
    chmod 755 "${INSTALL_DIR}/dots-terminal-filter"
    echo "Binary installed to ${INSTALL_DIR}/dots-terminal-filter"
}

main() {
    local install_type="user"
    local shell_type="all"
    local uninstall=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --user)
                install_type="user"
                shift
                ;;
            --system)
                install_type="system"
                shift
                ;;
            --shell)
                shell_type="$2"
                shift 2
                ;;
            --uninstall)
                uninstall=true
                shift
                ;;
            --help)
                show_help
                exit 0
                ;;
            *)
                echo "Unknown option: $1" >&2
                show_help
                exit 1
                ;;
        esac
    done
    
    if [[ "$uninstall" == true ]]; then
        uninstall_integration
        exit 0
    fi
    
    if [[ "$install_type" == "system" && $EUID -ne 0 ]]; then
        echo "Error: System installation requires root privileges" >&2
        echo "Please run with sudo or use --user for user installation" >&2
        exit 1
    fi
    
    install_binary
    
    local target_dir
    if [[ "$install_type" == "system" ]]; then
        target_dir="$SYSTEM_SHELL_DIR"
    else
        target_dir="$USER_SHELL_DIR"
    fi
    
    case "$shell_type" in
        bash)
            install_bash_integration "$target_dir"
            ;;
        zsh)
            install_zsh_integration "$target_dir"
            ;;
        fish)
            install_fish_integration "$target_dir"
            ;;
        all)
            install_bash_integration "$target_dir"
            install_zsh_integration "$target_dir"
            install_fish_integration "$target_dir"
            ;;
        *)
            echo "Error: Unknown shell type '$shell_type'. Supported: bash, zsh, fish, all" >&2
            exit 1
            ;;
    esac
    
    echo ""
    echo "DOTS Family Mode shell integration installed successfully!"
    echo ""
    echo "Next steps:"
    echo "1. Restart your shell or source your shell configuration file"
    echo "2. Commands will now be filtered through DOTS Family Mode"
    echo "3. Use 'dots_filter_disable' to temporarily disable filtering (requires parent password)"
    echo "4. Use 'dots_filter_enable' to re-enable filtering"
    echo ""
    echo "For support, visit: https://github.com/dots-distro/dots-family-mode"
}

main "$@"