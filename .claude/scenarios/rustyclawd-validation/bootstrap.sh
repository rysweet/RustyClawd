#!/bin/bash
# bootstrap.sh - Fix RustyClawd build environment by installing dependencies
#
# Philosophy: Ruthless simplicity
# - Detect OS and package manager
# - Install OpenSSL dev packages
# - Verify cargo build works
# - Generate results artifact
#
# Exit codes:
#   0 - Success (build works)
#   1 - Failure (installation failed or build broken)

set -euo pipefail

# Configuration
PROJECT_ROOT="${PROJECT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)}"
ARTIFACTS_DIR="${ARTIFACTS_DIR:-$PROJECT_ROOT/.claude/scenarios/rustyclawd-validation/reports}"

# Flags
DRY_RUN=false
CHECK_ONLY=false
INSTALL=false
BUILD=false
VERBOSE=false

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command-line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --check-only)
                CHECK_ONLY=true
                shift
                ;;
            --install)
                INSTALL=true
                shift
                ;;
            --build)
                BUILD=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done
}

# Detect operating system
detect_os() {
    local os_type=""

    case "$(uname -s)" in
        Darwin*)
            os_type="macos"
            ;;
        Linux*)
            if grep -qi microsoft /proc/version 2>/dev/null; then
                os_type="wsl"
            elif [ -f /etc/debian_version ]; then
                os_type="debian"
            elif [ -f /etc/redhat-release ]; then
                os_type="redhat"
            else
                os_type="linux"
            fi
            ;;
        *)
            os_type="unknown"
            ;;
    esac

    echo "$os_type"
}

# Check if OpenSSL development packages are installed
check_openssl() {
    if pkg-config --exists openssl 2>/dev/null; then
        return 0
    else
        return 1
    fi
}

# Get OpenSSL version
get_openssl_version() {
    if check_openssl; then
        pkg-config --modversion openssl 2>/dev/null || echo "unknown"
    else
        echo "not installed"
    fi
}

# Get package names for OS
get_package_names() {
    local os_type=$1
    case "$os_type" in
        macos)
            echo "openssl@3"
            ;;
        debian|wsl)
            echo "libssl-dev pkg-config"
            ;;
        redhat)
            echo "openssl-devel"
            ;;
        *)
            echo "openssl-dev"
            ;;
    esac
}

# Get installation command
get_install_command() {
    local os_type=$1
    local packages=$2

    case "$os_type" in
        macos)
            echo "brew install $packages"
            ;;
        debian|wsl)
            echo "sudo apt-get install -y $packages"
            ;;
        redhat)
            echo "sudo dnf install -y $packages"
            ;;
        *)
            echo "# Install OpenSSL development packages for your system"
            ;;
    esac
}

# Install OpenSSL development packages
install_openssl() {
    local os_type=$(detect_os)
    local packages=$(get_package_names "$os_type")
    local install_cmd=$(get_install_command "$os_type" "$packages")

    if $DRY_RUN; then
        echo "[Bootstrap] Would run: $install_cmd"
        return 0
    fi

    echo "[Bootstrap] Installing OpenSSL dependencies..."
    echo "[Bootstrap] Running: $install_cmd"

    if eval "$install_cmd" 2>&1; then
        echo -e "${GREEN}[Bootstrap] ✓ Installation successful${NC}"
        return 0
    else
        echo -e "${RED}[Bootstrap] ✗ Installation failed${NC}"
        return 1
    fi
}

# Check if cargo is available
check_cargo() {
    if command -v cargo &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Build RustyClawd
build_rustyclawd() {
    echo "[Bootstrap] Testing cargo build..."

    if ! check_cargo; then
        echo -e "${RED}[Bootstrap] ✗ cargo not found${NC}"
        echo "[Bootstrap] Please install Rust: https://rustup.rs"
        return 1
    fi

    cd "$PROJECT_ROOT"

    if cargo build --release 2>&1 | tee /tmp/cargo_build.log; then
        if [ -f "$PROJECT_ROOT/target/release/claude" ] || grep -q "Finished release" /tmp/cargo_build.log; then
            echo -e "${GREEN}[Bootstrap] ✓ Build successful${NC}"
            return 0
        else
            echo -e "${RED}[Bootstrap] ✗ Build succeeded but binary 'claude' not found${NC}"
            return 1
        fi
    else
        echo -e "${RED}[Bootstrap] ✗ Build failed${NC}"
        cat /tmp/cargo_build.log
        return 1
    fi
}

# Verify cargo binary exists
verify_binary() {
    if [ -f "$PROJECT_ROOT/target/release/claude" ]; then
        echo "[Bootstrap] Binary verified: claude"
        return 0
    else
        echo "[Bootstrap] Warning: Binary not found at expected location"
        return 1
    fi
}

# Write bootstrap status to artifacts
write_status() {
    local status=$1
    local os_type=$(detect_os)
    local openssl_version=$(get_openssl_version)

    mkdir -p "$ARTIFACTS_DIR"

    cat > "$ARTIFACTS_DIR/bootstrap_status.md" <<EOF
# Bootstrap Status Report

**Generated**: $(date '+%Y-%m-%d %H:%M:%S')
**Status**: $status
**Operating System**: $os_type

## Environment

- **OpenSSL Version**: $openssl_version
- **Cargo**: $(check_cargo && cargo --version || echo "not found")
- **Project Root**: $PROJECT_ROOT

## Build Results

$(if [ "$status" = "SUCCESS" ]; then
    echo "Build completed successfully. RustyClawd is ready for validation."
else
    echo "Build failed or dependencies missing."
fi)

## Next Steps

$(if [ "$status" = "SUCCESS" ]; then
    echo "Run validation: \`./validate.sh\`"
else
    echo "Fix the issues above and re-run bootstrap."
fi)
EOF

    echo "[Bootstrap] Status written to: $ARTIFACTS_DIR/bootstrap_status.md"
}

# Main execution
main() {
    parse_args "$@"

    local os_type=$(detect_os)
    echo "[Bootstrap] Detecting OS: $os_type"

    # Check-only mode
    if $CHECK_ONLY; then
        if check_openssl; then
            echo -e "${GREEN}[Bootstrap] ✓ OpenSSL development packages installed${NC}"
            echo "[Bootstrap] OpenSSL version: $(get_openssl_version)"
            exit 0
        else
            echo -e "${RED}[Bootstrap] ✗ OpenSSL development packages not found${NC}"
            local packages=$(get_package_names "$os_type")
            echo "[Bootstrap] Missing packages: $packages"
            echo "[Bootstrap] Install with: $(get_install_command "$os_type" "$packages")"
            exit 1
        fi
    fi

    # Install mode
    if $INSTALL; then
        if ! check_openssl; then
            if ! install_openssl; then
                write_status "FAILED"
                exit 1
            fi
        else
            echo "[Bootstrap] OpenSSL already installed"
        fi
        exit 0
    fi

    # Build mode
    if $BUILD; then
        if ! check_openssl; then
            echo -e "${YELLOW}[Bootstrap] Warning: OpenSSL not detected${NC}"
        fi

        if build_rustyclawd; then
            verify_binary || true
            write_status "SUCCESS"
            exit 0
        else
            write_status "FAILED"
            exit 1
        fi
    fi

    # Dry-run mode: just show what would be done
    if $DRY_RUN; then
        if ! check_openssl; then
            echo "[Bootstrap] Would install OpenSSL development packages"
            local packages=$(get_package_names "$os_type")
            echo "[Bootstrap] Would run: $(get_install_command "$os_type" "$packages")"
        else
            echo "[Bootstrap] OpenSSL already installed, no action needed"
        fi
        echo "[Bootstrap] Would build RustyClawd"
        exit 0
    fi

    # Default: Full bootstrap (check, install if needed, build)
    if ! check_openssl; then
        echo "[Bootstrap] OpenSSL development packages not found"
        if ! install_openssl; then
            echo -e "${RED}[Bootstrap] Failed to install dependencies${NC}"
            local packages=$(get_package_names "$os_type")
            echo "[Bootstrap] Please manually install: $packages"
            echo "[Bootstrap] Command: $(get_install_command "$os_type" "$packages")"
            write_status "FAILED"
            exit 1
        fi
    else
        echo "[Bootstrap] OpenSSL development packages already installed"
        echo "[Bootstrap] Version: $(get_openssl_version)"
    fi

    # Build RustyClawd
    if build_rustyclawd; then
        verify_binary || true
        write_status "SUCCESS"
        echo -e "${GREEN}[Bootstrap] ✓ Bootstrap complete${NC}"
        exit 0
    else
        write_status "FAILED"
        echo -e "${RED}[Bootstrap] ✗ Bootstrap failed${NC}"
        exit 1
    fi
}

main "$@"
