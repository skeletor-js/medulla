#!/bin/sh
# Medulla installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/skeletor-js/medulla/main/install.sh | sh

set -e

REPO="skeletor-js/medulla"
INSTALL_DIR="${MEDULLA_INSTALL_DIR:-$HOME/.local/bin}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

info() {
    printf "${GREEN}info${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warn${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
    exit 1
}

# Detect OS
detect_os() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    case "$OS" in
        darwin) echo "darwin" ;;
        linux) echo "linux" ;;
        mingw*|msys*|cygwin*) echo "windows" ;;
        *) error "Unsupported OS: $OS" ;;
    esac
}

# Detect architecture
detect_arch() {
    ARCH=$(uname -m)
    case "$ARCH" in
        x86_64|amd64) echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *) error "Unsupported architecture: $ARCH" ;;
    esac
}

# Get the target triple
get_target() {
    OS=$(detect_os)
    ARCH=$(detect_arch)

    case "$OS" in
        darwin) echo "${ARCH}-apple-darwin" ;;
        linux) echo "${ARCH}-unknown-linux-gnu" ;;
        windows) echo "${ARCH}-pc-windows-msvc" ;;
    esac
}

# Get the latest version from GitHub
get_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name"' | \
        sed -E 's/.*"([^"]+)".*/\1/'
}

# Main installation
main() {
    info "Detecting platform..."
    TARGET=$(get_target)
    info "Platform: $TARGET"

    info "Fetching latest version..."
    VERSION=$(get_latest_version)
    if [ -z "$VERSION" ]; then
        error "Failed to fetch latest version. Check your internet connection."
    fi
    info "Version: $VERSION"

    # Determine file extension
    OS=$(detect_os)
    if [ "$OS" = "windows" ]; then
        EXT="zip"
    else
        EXT="tar.gz"
    fi

    URL="https://github.com/${REPO}/releases/download/${VERSION}/medulla-${VERSION}-${TARGET}.${EXT}"

    info "Downloading from: $URL"

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Download and extract
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT

    if [ "$OS" = "windows" ]; then
        curl -fsSL "$URL" -o "$TEMP_DIR/medulla.zip"
        unzip -q "$TEMP_DIR/medulla.zip" -d "$TEMP_DIR"
        mv "$TEMP_DIR/medulla.exe" "$INSTALL_DIR/"
    else
        curl -fsSL "$URL" | tar xz -C "$TEMP_DIR"
        mv "$TEMP_DIR/medulla" "$INSTALL_DIR/"
        chmod +x "$INSTALL_DIR/medulla"
    fi

    info "Installed medulla to $INSTALL_DIR/medulla"

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
            echo ""
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
            echo ""
            ;;
    esac

    info "Installation complete! Run 'medulla --help' to get started."
}

main "$@"
