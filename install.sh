#!/bin/bash
#
# Agent Skills Generator Installer
# https://github.com/AmanSikarwar/agent-skills-generator
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/AmanSikarwar/agent-skills-generator/master/install.sh | bash
#
# Environment variables:
#   INSTALL_DIR - Installation directory (default: ~/.local/bin or /usr/local/bin)
#   VERSION     - Specific version to install (default: latest)
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
REPO="AmanSikarwar/agent-skills-generator"
BINARY_NAME="agent-skills-generator"

# Print functions
info() {
    echo -e "${BLUE}INFO${NC} $1"
}

success() {
    echo -e "${GREEN}SUCCESS${NC} $1"
}

warn() {
    echo -e "${YELLOW}WARN${NC} $1"
}

error() {
    echo -e "${RED}ERROR${NC} $1" >&2
}

# Print banner
print_banner() {
    echo -e "${CYAN}"
    cat << 'EOF'
    _                    _     ____  _    _ _ _
   / \   __ _  ___ _ __ | |_  / ___|| | _(_) | |___
  / _ \ / _` |/ _ \ '_ \| __| \___ \| |/ / | | / __|
 / ___ \ (_| |  __/ | | | |_   ___) |   <| | | \__ \
/_/   \_\__, |\___|_| |_|\__| |____/|_|\_\_|_|_|___/
        |___/
              Generator Installer
EOF
    echo -e "${NC}"
}

# Detect OS
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
        *)       error "Unsupported operating system: $os"; exit 1 ;;
    esac
}

# Detect architecture
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $arch"; exit 1 ;;
    esac
}

# Get the target triple
get_target() {
    local os arch
    os="$(detect_os)"
    arch="$(detect_arch)"

    case "$os" in
        linux)
            echo "${arch}-unknown-linux-gnu"
            ;;
        darwin)
            echo "${arch}-apple-darwin"
            ;;
        windows)
            echo "${arch}-pc-windows-msvc"
            ;;
    esac
}

# Get latest version from GitHub
get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

    if [[ -z "$version" ]]; then
        error "Failed to fetch latest version"
        exit 1
    fi

    echo "$version"
}

# Get download URL
get_download_url() {
    local version target extension
    version="$1"
    target="$2"

    if [[ "$target" == *"windows"* ]]; then
        extension="zip"
    else
        extension="tar.gz"
    fi

    echo "https://github.com/${REPO}/releases/download/${version}/${BINARY_NAME}-${version}-${target}.${extension}"
}

# Determine installation directory
get_install_dir() {
    if [[ -n "${INSTALL_DIR:-}" ]]; then
        echo "$INSTALL_DIR"
    elif [[ -w "/usr/local/bin" ]]; then
        echo "/usr/local/bin"
    elif [[ -d "$HOME/.local/bin" ]]; then
        echo "$HOME/.local/bin"
    else
        mkdir -p "$HOME/.local/bin"
        echo "$HOME/.local/bin"
    fi
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Download and install
install() {
    local version target url install_dir tmp_dir archive_name

    print_banner

    # Detect system
    info "Detecting system..."
    target="$(get_target)"
    info "Target: ${BOLD}${target}${NC}"

    # Get version
    if [[ -n "${VERSION:-}" ]]; then
        version="$VERSION"
        info "Using specified version: ${BOLD}${version}${NC}"
    else
        info "Fetching latest version..."
        version="$(get_latest_version)"
        info "Latest version: ${BOLD}${version}${NC}"
    fi

    # Get install directory
    install_dir="$(get_install_dir)"
    info "Installation directory: ${BOLD}${install_dir}${NC}"

    # Get download URL
    url="$(get_download_url "$version" "$target")"
    info "Downloading from: ${BOLD}${url}${NC}"

    # Create temporary directory
    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT

    # Download archive
    info "Downloading..."
    if [[ "$target" == *"windows"* ]]; then
        archive_name="${BINARY_NAME}.zip"
    else
        archive_name="${BINARY_NAME}.tar.gz"
    fi

    if command_exists curl; then
        curl -fsSL "$url" -o "${tmp_dir}/${archive_name}"
    elif command_exists wget; then
        wget -q "$url" -O "${tmp_dir}/${archive_name}"
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    # Extract archive
    info "Extracting..."
    cd "$tmp_dir"
    if [[ "$target" == *"windows"* ]]; then
        if command_exists unzip; then
            unzip -q "$archive_name"
        else
            error "unzip not found. Please install it."
            exit 1
        fi
    else
        tar -xzf "$archive_name"
    fi

    # Install binary
    info "Installing..."
    if [[ -w "$install_dir" ]]; then
        mv "${BINARY_NAME}" "${install_dir}/"
        chmod +x "${install_dir}/${BINARY_NAME}"
    else
        warn "Need elevated permissions to install to ${install_dir}"
        sudo mv "${BINARY_NAME}" "${install_dir}/"
        sudo chmod +x "${install_dir}/${BINARY_NAME}"
    fi

    # Verify installation
    if command_exists "$BINARY_NAME"; then
        success "Successfully installed ${BOLD}${BINARY_NAME}${NC} ${version}"
        echo ""
        info "Run '${BOLD}${BINARY_NAME} --help${NC}' to get started"
    else
        warn "Installation complete, but ${BINARY_NAME} is not in PATH"
        echo ""
        info "Add the following to your shell profile:"
        echo -e "    ${BOLD}export PATH=\"\$PATH:${install_dir}\"${NC}"
    fi

    # Check if install_dir is in PATH
    if [[ ":$PATH:" != *":${install_dir}:"* ]]; then
        echo ""
        warn "${install_dir} is not in your PATH"
        info "Add it to your shell profile:"
        echo ""
        echo -e "    ${BOLD}# For bash (~/.bashrc or ~/.bash_profile)${NC}"
        echo -e "    export PATH=\"\$PATH:${install_dir}\""
        echo ""
        echo -e "    ${BOLD}# For zsh (~/.zshrc)${NC}"
        echo -e "    export PATH=\"\$PATH:${install_dir}\""
        echo ""
        echo -e "    ${BOLD}# For fish (~/.config/fish/config.fish)${NC}"
        echo -e "    fish_add_path ${install_dir}"
    fi
}

# Run installer
install
