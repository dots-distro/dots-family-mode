#!/usr/bin/env bash
# DOTS Family Mode Shell Integration Installer
# Automatically sets up terminal filtering for bash, zsh, and fish

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${DOTS_INSTALL_DIR:-/usr/local/share/dots-family}"
USER_HOME="${HOME:-$(eval echo ~$USER)}"

print_usage() {
    cat <<EOF
DOTS Family Mode Shell Integration Installer

Usage: $0 [OPTIONS]

OPTIONS:
  --shell SHELL       Install for specific shell (bash|zsh|fish|all)
  --user              Install for current user only (default)
  --system            Install system-wide
  --uninstall         Remove shell integration
  --dry-run           Show what would be done without making changes
  --help              Show this help message

EXAMPLES:
  $0                           # Install for all shells, current user
  $0 --shell bash              # Install for bash only
  $0 --system                  # Install system-wide
  $0 --uninstall               # Remove integration

ENVIRONMENT VARIABLES:
  DOTS_INSTALL_DIR    - Installation directory (default: /usr/local/share/dots-family)
  DOTS_FILTER_BIN     - Path to filter binary (default: dots-terminal-filter)
EOF
}

log() {
    echo "[DOTS-INSTALL] $*" >&2
}

error() {
    log "ERROR: $*"
    exit 1
}

check_requirements() {
    if ! command -v dots-terminal-filter >/dev/null 2>&1; then
        error "dots-terminal-filter not found in PATH. Please install DOTS Family Mode first."
    fi
    
    log "dots-terminal-filter found: $(command -v dots-terminal-filter)"
}

detect_shells() {
    local shells=()
    
    if command -v bash >/dev/null 2>&1; then
        shells+=("bash")
    fi
    
    if command -v zsh >/dev/null 2>&1; then
        shells+=("zsh") 
    fi
    
    if command -v fish >/dev/null 2>&1; then
        shells+=("fish")
    fi
    
    echo "${shells[@]}"
}

install_for_bash() {
    local rc_file="$1"
    local integration_path="$2"
    
    log "Installing bash integration to $rc_file"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log "Would add to $rc_file:"
        log "  source $integration_path"
        log "  dots_filter_setup"
        return 0
    fi
    
    # Check if already installed
    if grep -q "dots-bash-integration.sh" "$rc_file" 2>/dev/null; then
        log "Bash integration already present in $rc_file"
        return 0
    fi
    
    # Add integration
    cat >> "$rc_file" <<EOF

# DOTS Family Mode Terminal Filtering
if [ -f "$integration_path" ]; then
    source "$integration_path"
    dots_filter_setup
fi
EOF
    
    log "Bash integration added to $rc_file"
}

install_for_zsh() {
    local rc_file="$1"
    local integration_path="$2"
    
    log "Installing zsh integration to $rc_file"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log "Would add to $rc_file:"
        log "  source $integration_path"
        log "  dots_filter_setup"
        return 0
    fi
    
    # Check if already installed
    if grep -q "dots-zsh-integration.sh" "$rc_file" 2>/dev/null; then
        log "Zsh integration already present in $rc_file"
        return 0
    fi
    
    # Add integration
    cat >> "$rc_file" <<EOF

# DOTS Family Mode Terminal Filtering
if [[ -f "$integration_path" ]]; then
    source "$integration_path"
    dots_filter_setup
fi
EOF
    
    log "Zsh integration added to $rc_file"
}

install_for_fish() {
    local config_file="$1"
    local integration_path="$2"
    
    log "Installing fish integration to $config_file"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log "Would add to $config_file:"
        log "  source $integration_path"
        log "  dots_filter_setup"
        return 0
    fi
    
    # Create config directory if needed
    local config_dir="$(dirname "$config_file")"
    mkdir -p "$config_dir"
    
    # Check if already installed
    if grep -q "dots-fish-integration.fish" "$config_file" 2>/dev/null; then
        log "Fish integration already present in $config_file"
        return 0
    fi
    
    # Add integration
    cat >> "$config_file" <<EOF

# DOTS Family Mode Terminal Filtering
if test -f "$integration_path"
    source "$integration_path"
    dots_filter_setup
end
EOF
    
    log "Fish integration added to $config_file"
}

uninstall_from_file() {
    local file="$1"
    local pattern="$2"
    
    if [[ ! -f "$file" ]]; then
        return 0
    fi
    
    if [[ "$DRY_RUN" == "true" ]]; then
        if grep -q "$pattern" "$file"; then
            log "Would remove DOTS integration from $file"
        fi
        return 0
    fi
    
    if grep -q "$pattern" "$file"; then
        cp "$file" "${file}.dots-backup"
        sed -i '/# DOTS Family Mode Terminal Filtering/,/^$/d' "$file"
        log "Removed DOTS integration from $file (backup: ${file}.dots-backup)"
    fi
}

install_integration() {
    local target_shell="$1"
    
    case "$target_shell" in
        bash)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                install_for_bash "/etc/bash.bashrc" "$INSTALL_DIR/dots-bash-integration.sh"
            else
                install_for_bash "$USER_HOME/.bashrc" "$INSTALL_DIR/dots-bash-integration.sh"
            fi
            ;;
        zsh)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                install_for_zsh "/etc/zsh/zshrc" "$INSTALL_DIR/dots-zsh-integration.sh"
            else
                install_for_zsh "$USER_HOME/.zshrc" "$INSTALL_DIR/dots-zsh-integration.sh"
            fi
            ;;
        fish)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                install_for_fish "/etc/fish/config.fish" "$INSTALL_DIR/dots-fish-integration.fish"
            else
                install_for_fish "$USER_HOME/.config/fish/config.fish" "$INSTALL_DIR/dots-fish-integration.fish"
            fi
            ;;
        *)
            error "Unsupported shell: $target_shell"
            ;;
    esac
}

uninstall_integration() {
    local target_shell="$1"
    
    case "$target_shell" in
        bash)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                uninstall_from_file "/etc/bash.bashrc" "DOTS Family Mode"
            else
                uninstall_from_file "$USER_HOME/.bashrc" "DOTS Family Mode"
            fi
            ;;
        zsh)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                uninstall_from_file "/etc/zsh/zshrc" "DOTS Family Mode"
            else
                uninstall_from_file "$USER_HOME/.zshrc" "DOTS Family Mode"
            fi
            ;;
        fish)
            if [[ "$SYSTEM_INSTALL" == "true" ]]; then
                uninstall_from_file "/etc/fish/config.fish" "DOTS Family Mode"
            else
                uninstall_from_file "$USER_HOME/.config/fish/config.fish" "DOTS Family Mode"
            fi
            ;;
        *)
            error "Unsupported shell: $target_shell"
            ;;
    esac
}

copy_integration_files() {
    if [[ "$DRY_RUN" == "true" ]]; then
        log "Would copy integration files to $INSTALL_DIR"
        return 0
    fi
    
    if [[ "$SYSTEM_INSTALL" == "true" ]] && [[ $EUID -ne 0 ]]; then
        error "System install requires root privileges. Use sudo."
    fi
    
    mkdir -p "$INSTALL_DIR"
    
    cp "$SCRIPT_DIR/dots-bash-integration.sh" "$INSTALL_DIR/"
    cp "$SCRIPT_DIR/dots-zsh-integration.sh" "$INSTALL_DIR/"
    cp "$SCRIPT_DIR/dots-fish-integration.fish" "$INSTALL_DIR/"
    
    chmod 644 "$INSTALL_DIR"/dots-*-integration.*
    
    log "Integration files copied to $INSTALL_DIR"
}

main() {
    local target_shells=()
    local uninstall=false
    local system_install=false
    local dry_run=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --shell)
                if [[ -n "${2:-}" ]]; then
                    if [[ "$2" == "all" ]]; then
                        target_shells=($(detect_shells))
                    else
                        target_shells=("$2")
                    fi
                    shift 2
                else
                    error "--shell requires an argument"
                fi
                ;;
            --user)
                system_install=false
                shift
                ;;
            --system)
                system_install=true
                shift
                ;;
            --uninstall)
                uninstall=true
                shift
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                ;;
        esac
    done
    
    # Set global variables
    SYSTEM_INSTALL="$system_install"
    DRY_RUN="$dry_run"
    
    # Default to all detected shells
    if [[ ${#target_shells[@]} -eq 0 ]]; then
        target_shells=($(detect_shells))
    fi
    
    log "Target shells: ${target_shells[*]}"
    log "System install: $system_install"
    log "Dry run: $dry_run"
    
    if [[ "$uninstall" == "true" ]]; then
        log "Uninstalling DOTS Family Mode shell integration..."
        for shell in "${target_shells[@]}"; do
            uninstall_integration "$shell"
        done
        log "Uninstall complete"
    else
        log "Installing DOTS Family Mode shell integration..."
        check_requirements
        copy_integration_files
        
        for shell in "${target_shells[@]}"; do
            install_integration "$shell"
        done
        
        log "Installation complete"
        log ""
        log "Please restart your shell or run 'source ~/.bashrc' (or equivalent)"
        log "to activate terminal filtering."
    fi
}

main "$@"