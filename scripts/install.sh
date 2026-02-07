#!/bin/bash
# Copyright 2026 Aptove
# SPDX-License-Identifier: Apache-2.0
#
# xcbridge install script
# Installs xcbridge as a macOS LaunchAgent service

set -euo pipefail

# Configuration
BINARY_NAME="xcbridge"
INSTALL_DIR="${HOME}/.local/bin"
PLIST_DIR="${HOME}/Library/LaunchAgents"
PLIST_NAME="ai.aptove.xcbridge.plist"
DEFAULT_PORT=9090

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
        error "xcbridge only runs on macOS"
        exit 1
    fi
}

# Check if Xcode is installed
check_xcode() {
    if ! command -v xcodebuild &> /dev/null; then
        error "Xcode command line tools not found"
        error "Install Xcode from the App Store or run: xcode-select --install"
        exit 1
    fi
    info "Xcode found: $(xcodebuild -version | head -1)"
}

# Find the binary to install
find_binary() {
    local binary_path=""
    
    # Check if binary path was provided as argument
    if [[ $# -ge 1 && -f "$1" ]]; then
        binary_path="$1"
    # Check current directory
    elif [[ -f "./${BINARY_NAME}" ]]; then
        binary_path="./${BINARY_NAME}"
    # Check target/release (Cargo build output)
    elif [[ -f "./target/release/${BINARY_NAME}" ]]; then
        binary_path="./target/release/${BINARY_NAME}"
    # Check target/debug
    elif [[ -f "./target/debug/${BINARY_NAME}" ]]; then
        binary_path="./target/debug/${BINARY_NAME}"
        warn "Using debug build - consider using release build for better performance"
    else
        error "Binary not found. Build with: cargo build --release"
        exit 1
    fi
    
    echo "$binary_path"
}

# Install the binary
install_binary() {
    local binary_path="$1"
    
    mkdir -p "${INSTALL_DIR}"
    
    info "Installing ${BINARY_NAME} to ${INSTALL_DIR}"
    cp "${binary_path}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    
    # Add to PATH if not already there
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        warn "${INSTALL_DIR} is not in your PATH"
        warn "Add the following to your shell profile:"
        warn "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    fi
}

# Create LaunchAgent plist
create_plist() {
    local port="${1:-$DEFAULT_PORT}"
    local api_key="${2:-}"
    
    mkdir -p "${PLIST_DIR}"
    
    info "Creating LaunchAgent plist"
    
    local args="<string>--port</string>
        <string>${port}</string>"
    
    if [[ -n "${api_key}" ]]; then
        args="${args}
        <string>--api-key</string>
        <string>${api_key}</string>"
    fi
    
    cat > "${PLIST_DIR}/${PLIST_NAME}" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>ai.aptove.xcbridge</string>
    
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/${BINARY_NAME}</string>
        ${args}
    </array>
    
    <key>RunAtLoad</key>
    <true/>
    
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    
    <key>StandardOutPath</key>
    <string>${HOME}/Library/Logs/xcbridge.log</string>
    
    <key>StandardErrorPath</key>
    <string>${HOME}/Library/Logs/xcbridge.error.log</string>
    
    <key>WorkingDirectory</key>
    <string>${HOME}</string>
    
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOF
}

# Load the LaunchAgent
load_service() {
    info "Loading xcbridge service"
    
    # Unload if already loaded
    launchctl unload "${PLIST_DIR}/${PLIST_NAME}" 2>/dev/null || true
    
    # Load the service
    launchctl load "${PLIST_DIR}/${PLIST_NAME}"
    
    # Wait a moment for the service to start
    sleep 2
    
    # Check if service is running
    if pgrep -x "${BINARY_NAME}" > /dev/null; then
        info "xcbridge service started successfully"
    else
        error "Failed to start xcbridge service"
        error "Check logs at: ${HOME}/Library/Logs/xcbridge.error.log"
        exit 1
    fi
}

# Print usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] [BINARY_PATH]

Install xcbridge as a macOS LaunchAgent service.

Options:
    -p, --port PORT      Port to listen on (default: ${DEFAULT_PORT})
    -k, --api-key KEY    API key for authentication (optional)
    -h, --help           Show this help message

Arguments:
    BINARY_PATH          Path to xcbridge binary (optional, auto-detected)

Examples:
    $0                              # Auto-detect binary, default settings
    $0 -p 8080                      # Use custom port
    $0 -k my-secret-key             # Enable API key authentication
    $0 ./target/release/xcbridge    # Specify binary path

EOF
}

# Main
main() {
    local port="${DEFAULT_PORT}"
    local api_key=""
    local binary_path=""
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -p|--port)
                port="$2"
                shift 2
                ;;
            -k|--api-key)
                api_key="$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            -*)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                binary_path="$1"
                shift
                ;;
        esac
    done
    
    info "Installing xcbridge..."
    
    check_platform
    check_xcode
    
    if [[ -z "${binary_path}" ]]; then
        binary_path=$(find_binary)
    fi
    
    info "Using binary: ${binary_path}"
    
    install_binary "${binary_path}"
    create_plist "${port}" "${api_key}"
    load_service
    
    echo ""
    info "Installation complete!"
    info "xcbridge is running on http://127.0.0.1:${port}"
    echo ""
    info "Useful commands:"
    info "  View logs:     tail -f ~/Library/Logs/xcbridge.log"
    info "  Stop service:  launchctl unload ~/Library/LaunchAgents/${PLIST_NAME}"
    info "  Start service: launchctl load ~/Library/LaunchAgents/${PLIST_NAME}"
    info "  Uninstall:     ./scripts/uninstall.sh"
}

main "$@"
