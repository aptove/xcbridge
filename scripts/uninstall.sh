#!/bin/bash
# Copyright 2026 Aptove
# SPDX-License-Identifier: Apache-2.0
#
# xcbridge uninstall script
# Removes xcbridge service and binary from the system

set -euo pipefail

# Configuration
BINARY_NAME="xcbridge"
INSTALL_DIR="${HOME}/.local/bin"
PLIST_DIR="${HOME}/Library/LaunchAgents"
PLIST_NAME="ai.aptove.xcbridge.plist"
LOG_DIR="${HOME}/Library/Logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() { echo -e "${GREEN}[INFO]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check if running on macOS
check_platform() {
    if [[ "$(uname)" != "Darwin" ]]; then
        error "This script only runs on macOS"
        exit 1
    fi
}

# Stop the service
stop_service() {
    local plist_path="${PLIST_DIR}/${PLIST_NAME}"
    
    if [[ -f "${plist_path}" ]]; then
        info "Stopping xcbridge service..."
        launchctl unload "${plist_path}" 2>/dev/null || true
        
        # Wait for process to stop
        local attempts=0
        while pgrep -x "${BINARY_NAME}" > /dev/null && [[ $attempts -lt 10 ]]; do
            sleep 1
            ((attempts++))
        done
        
        # Force kill if still running
        if pgrep -x "${BINARY_NAME}" > /dev/null; then
            warn "Service did not stop gracefully, forcing termination"
            pkill -9 -x "${BINARY_NAME}" 2>/dev/null || true
        fi
        
        info "Service stopped"
    else
        info "No service plist found"
    fi
}

# Remove files
remove_files() {
    local removed=false
    
    # Remove binary
    local binary_path="${INSTALL_DIR}/${BINARY_NAME}"
    if [[ -f "${binary_path}" ]]; then
        info "Removing binary: ${binary_path}"
        rm -f "${binary_path}"
        removed=true
    fi
    
    # Remove plist
    local plist_path="${PLIST_DIR}/${PLIST_NAME}"
    if [[ -f "${plist_path}" ]]; then
        info "Removing plist: ${plist_path}"
        rm -f "${plist_path}"
        removed=true
    fi
    
    if [[ "${removed}" = false ]]; then
        info "No xcbridge files found to remove"
    fi
}

# Optionally remove logs
remove_logs() {
    local log_file="${LOG_DIR}/xcbridge.log"
    local error_log="${LOG_DIR}/xcbridge.error.log"
    
    if [[ -f "${log_file}" ]] || [[ -f "${error_log}" ]]; then
        echo ""
        read -p "Remove log files? [y/N] " -n 1 -r
        echo ""
        
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            [[ -f "${log_file}" ]] && rm -f "${log_file}" && info "Removed ${log_file}"
            [[ -f "${error_log}" ]] && rm -f "${error_log}" && info "Removed ${error_log}"
        else
            info "Log files preserved"
        fi
    fi
}

# Print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Uninstall xcbridge service and remove all related files.

Options:
    -f, --force     Don't prompt for log file removal (removes them)
    -k, --keep-logs Don't prompt for log file removal (keeps them)
    -h, --help      Show this help message

EOF
}

# Main
main() {
    local force=false
    local keep_logs=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -f|--force)
                force=true
                shift
                ;;
            -k|--keep-logs)
                keep_logs=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
    
    info "Uninstalling xcbridge..."
    
    check_platform
    stop_service
    remove_files
    
    # Handle logs based on flags
    if [[ "${force}" = true ]]; then
        rm -f "${LOG_DIR}/xcbridge.log" 2>/dev/null || true
        rm -f "${LOG_DIR}/xcbridge.error.log" 2>/dev/null || true
        info "Log files removed"
    elif [[ "${keep_logs}" = false ]]; then
        remove_logs
    fi
    
    echo ""
    info "Uninstall complete!"
}

main "$@"
