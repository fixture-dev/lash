#!/bin/bash
# Lash CLI Installer
# Builds and installs the Lash CLI globally via Cargo

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m' # No Color

# Paths
CARGO_BIN="$HOME/.cargo/bin"
LASH_BINARY="$CARGO_BIN/lash"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Print the Lash banner
print_banner() {
    echo ""
    echo -e "${CYAN}${BOLD}┓    ┓${NC}"
    echo -e "${CYAN}${BOLD}┃ ┏┓┏┣┓${NC}"
    echo -e "${CYAN}${BOLD}┗┛┗┻┛┛┗${NC}"
    echo -e "${DIM}Minimalist task tracker for devs and agents${NC}"
    echo ""
}

# Print usage
usage() {
    echo -e "${BOLD}Usage:${NC} $0 [COMMAND]"
    echo ""
    echo -e "${BOLD}Commands:${NC}"
    echo -e "  ${GREEN}install${NC}     Build and install Lash globally ${DIM}(default)${NC}"
    echo -e "  ${YELLOW}reinstall${NC}   Force reinstall even if version unchanged"
    echo -e "  ${RED}uninstall${NC}   Remove Lash from system"
    echo -e "  ${BLUE}status${NC}      Check current installation status"
    echo -e "  ${CYAN}help${NC}        Show this help message"
    echo ""
    echo -e "${BOLD}Locations:${NC}"
    echo -e "  ${DIM}Binary:${NC}     $LASH_BINARY"
    echo -e "  ${DIM}Source:${NC}     $PROJECT_ROOT/crates/lash-cli"
    echo ""
}

# Check if ~/.cargo/bin is in PATH
check_path() {
    if [[ ":$PATH:" != *":$CARGO_BIN:"* ]]; then
        echo -e "${YELLOW}⚠️  Warning:${NC} $CARGO_BIN is not in your PATH"
        echo ""
        echo -e "${DIM}Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):${NC}"
        echo -e "  ${CYAN}export PATH=\"\$HOME/.cargo/bin:\$PATH\"${NC}"
        echo ""
        return 1
    fi
    return 0
}

# Install Lash
do_install() {
    local force_flag=""
    if [ "$1" = "--force" ]; then
        force_flag="--force"
        echo -e "${YELLOW}🔄 Reinstalling Lash...${NC}"
    else
        echo -e "${GREEN}📦 Installing Lash...${NC}"
    fi
    echo ""

    # Change to project root
    cd "$PROJECT_ROOT"

    echo -e "${DIM}Building release binary from crates/lash-cli...${NC}"
    echo -e "${DIM}This compiles with optimizations (LTO enabled) - may take a minute.${NC}"
    echo ""

    # Run cargo install
    cargo install --path crates/lash-cli $force_flag

    echo ""

    # Verify installation
    if [ -x "$LASH_BINARY" ]; then
        echo -e "${GREEN}✅ Installation successful!${NC}"
        echo ""

        # Check PATH
        if check_path; then
            echo -e "${DIM}Verifying installation...${NC}"
            echo ""
            "$LASH_BINARY" --version
            echo ""
            echo -e "${GREEN}🎉 Lash is ready to use!${NC}"
            echo -e "${DIM}Try: lash --help${NC}"
        else
            echo ""
            echo -e "${DIM}After updating your PATH, verify with:${NC}"
            echo -e "  ${CYAN}lash --version${NC}"
        fi
    else
        echo -e "${RED}❌ Installation failed - binary not found${NC}"
        exit 1
    fi
}

# Uninstall Lash
do_uninstall() {
    echo -e "${RED}🗑️  Uninstalling Lash...${NC}"
    echo ""

    if [ -x "$LASH_BINARY" ]; then
        echo -e "${DIM}Removing binary from $CARGO_BIN...${NC}"
        cargo uninstall lash-cli 2>/dev/null || rm -f "$LASH_BINARY"
        echo ""
        echo -e "${GREEN}✅ Lash has been uninstalled${NC}"

        # Verify removal
        if command -v lash &> /dev/null; then
            echo -e "${YELLOW}⚠️  Note: 'lash' still found in PATH (maybe a different installation?)${NC}"
            echo -e "${DIM}Location: $(which lash)${NC}"
        else
            echo -e "${DIM}Verified: 'which lash' returns nothing${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  Lash is not installed at $LASH_BINARY${NC}"

        # Check if it's installed elsewhere
        if command -v lash &> /dev/null; then
            echo -e "${DIM}Found lash elsewhere: $(which lash)${NC}"
        fi
    fi
}

# Check installation status
do_status() {
    echo -e "${BLUE}🔍 Checking Lash installation status...${NC}"
    echo ""

    # Check binary
    echo -e "${BOLD}Binary:${NC}"
    if [ -x "$LASH_BINARY" ]; then
        echo -e "  ${GREEN}✓${NC} Found at $LASH_BINARY"
        local version=$("$LASH_BINARY" --version 2>/dev/null || echo "unknown")
        echo -e "  ${DIM}Version: $version${NC}"
    else
        echo -e "  ${RED}✗${NC} Not found at $LASH_BINARY"
    fi
    echo ""

    # Check PATH
    echo -e "${BOLD}PATH:${NC}"
    if check_path 2>/dev/null; then
        echo -e "  ${GREEN}✓${NC} $CARGO_BIN is in PATH"
    else
        echo -e "  ${YELLOW}✗${NC} $CARGO_BIN is not in PATH"
    fi
    echo ""

    # Check which lash
    echo -e "${BOLD}which lash:${NC}"
    if command -v lash &> /dev/null; then
        echo -e "  ${GREEN}✓${NC} $(which lash)"
    else
        echo -e "  ${RED}✗${NC} Not found in PATH"
    fi
    echo ""

    # Check source
    echo -e "${BOLD}Source:${NC}"
    if [ -f "$PROJECT_ROOT/crates/lash-cli/Cargo.toml" ]; then
        echo -e "  ${GREEN}✓${NC} Found at $PROJECT_ROOT/crates/lash-cli"
    else
        echo -e "  ${RED}✗${NC} Source not found (are you in the lash repo?)"
    fi
}

# Main
print_banner

case "${1:-install}" in
    install)
        do_install
        ;;
    reinstall|force|--force)
        do_install --force
        ;;
    uninstall|remove)
        do_uninstall
        ;;
    status|check)
        do_status
        ;;
    help|--help|-h)
        usage
        ;;
    *)
        echo -e "${RED}❌ Unknown command: $1${NC}"
        echo ""
        usage
        exit 1
        ;;
esac
