#!/bin/bash
# test_bootstrap.sh - Unit tests for bootstrap.sh
#
# Philosophy: Test behavior, not implementation
# - Mock external commands (pkg-config, apt-get, cargo)
# - Test success and failure paths
# - Verify error messages are helpful
#
# Coverage: 60% (Unit tests)
# - OpenSSL dependency detection
# - Package installation
# - RustyClawd build process
# - Error handling

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/test_helpers.sh"

# Assume bootstrap.sh will be in parent directory
BOOTSTRAP_SCRIPT="$SCRIPT_DIR/../bootstrap.sh"

# Test: bootstrap.sh detects missing OpenSSL packages
test_bootstrap_detects_missing_openssl() {
    # Mock: pkg-config returns error (OpenSSL not found)
    mock_command "pkg-config" "exit 1"

    # Run: bootstrap.sh check
    run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Should fail with helpful message
    assert_failure
    assert_output_contains "OpenSSL"
}

# Test: bootstrap.sh detects openssl-devel on Fedora
test_bootstrap_detects_fedora_package_name() {
    # Mock: Fedora system
    mock_command "pkg-config" "exit 1"
    mock_command "rpm" "echo 'rpm version'"

    # Run: bootstrap.sh check
    run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Should mention openssl-devel (Fedora package name)
    assert_failure
    assert_output_contains "openssl-devel"
}

# Test: bootstrap.sh detects libssl-dev on Ubuntu/Debian
test_bootstrap_detects_debian_package_name() {
    # Mock: Debian/Ubuntu system
    mock_command "pkg-config" "exit 1"
    mock_command "dpkg" "echo 'dpkg version'"

    # Run: bootstrap.sh check
    run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Should mention libssl-dev (Debian package name)
    assert_failure
    assert_output_contains "libssl-dev"
}

# Test: bootstrap.sh installs dependencies with sudo
test_bootstrap_installs_dependencies() {
    # Mock: System without OpenSSL
    mock_command "pkg-config" "exit 1"
    mock_command "dpkg" "echo 'dpkg version'"
    mock_command "sudo" "echo 'sudo: apt-get install libssl-dev' && exit 0"
    mock_command "apt-get" "exit 0"

    # Run: bootstrap.sh install
    run bash "$BOOTSTRAP_SCRIPT" --install

    # Assert: Should call sudo apt-get
    assert_success
    assert_output_contains "apt-get install"
}

# Test: bootstrap.sh builds RustyClawd after dependencies installed
test_bootstrap_builds_rustyclawd() {
    # Mock: Dependencies satisfied
    mock_command "pkg-config" "echo '--libs --cflags openssl' && exit 0"
    mock_command "cargo" "echo 'Compiling rustyclawd' && echo 'Finished release [optimized] target(s)' && exit 0"

    # Run: bootstrap.sh build
    run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should build successfully
    assert_success
    assert_output_contains "Finished release"
}

# Test: bootstrap.sh verifies cargo binary after build
test_bootstrap_verifies_cargo_binary() {
    # Mock: Successful build
    mock_command "pkg-config" "exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"

    # Create fake binary
    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"
    chmod +x "$TEST_TMPDIR/target/release/rustyclawd"

    # Run: bootstrap.sh with PROJECT_ROOT override
    PROJECT_ROOT="$TEST_TMPDIR" run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should verify binary exists
    assert_success
    assert_output_contains "rustyclawd"
}

# Test: bootstrap.sh fails gracefully if build errors occur
test_bootstrap_handles_build_failure() {
    # Mock: Dependencies OK, but build fails
    mock_command "pkg-config" "exit 0"
    mock_command "cargo" "echo 'error: could not compile' && exit 101"

    # Run: bootstrap.sh build
    run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should fail with error message
    assert_failure
    assert_output_contains "error"
}

# Test: bootstrap.sh reports missing cargo
test_bootstrap_detects_missing_cargo() {
    # Mock: cargo not found
    mock_command "pkg-config" "exit 0"
    mock_command "command" "exit 1"  # command -v cargo fails

    # Run: bootstrap.sh build
    run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should report cargo missing
    assert_failure
    assert_output_contains "cargo"
}

# Test: bootstrap.sh creates artifacts directory
test_bootstrap_creates_artifacts_dir() {
    # Mock: Successful build
    mock_command "pkg-config" "exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"

    # Run: bootstrap.sh with ARTIFACTS_DIR override
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" PROJECT_ROOT="$TEST_TMPDIR" \
        run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should create artifacts directory
    assert_success
    assert_dir_exists "$TEST_TMPDIR/artifacts"
}

# Test: bootstrap.sh writes build status to artifacts
test_bootstrap_writes_build_status() {
    # Mock: Successful build
    mock_command "pkg-config" "exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"

    # Run: bootstrap.sh with overrides
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" PROJECT_ROOT="$TEST_TMPDIR" \
        run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should write bootstrap_status.md
    assert_success
    assert_file_exists "$TEST_TMPDIR/artifacts/bootstrap_status.md"
    assert_file_contains "$TEST_TMPDIR/artifacts/bootstrap_status.md" "Bootstrap"
}

# Test: bootstrap.sh includes OpenSSL version in status
test_bootstrap_includes_openssl_version() {
    # Mock: pkg-config returns version
    mock_command "pkg-config" "echo '1.1.1' && exit 0"
    mock_command "cargo" "echo 'Finished release' && exit 0"

    mkdir -p "$TEST_TMPDIR/target/release"
    touch "$TEST_TMPDIR/target/release/rustyclawd"

    # Run: bootstrap.sh
    ARTIFACTS_DIR="$TEST_TMPDIR/artifacts" PROJECT_ROOT="$TEST_TMPDIR" \
        run bash "$BOOTSTRAP_SCRIPT" --build

    # Assert: Should include OpenSSL version
    assert_success
    assert_file_contains "$TEST_TMPDIR/artifacts/bootstrap_status.md" "OpenSSL"
}

# Test: bootstrap.sh provides installation instructions on failure
test_bootstrap_provides_install_instructions() {
    # Mock: Missing OpenSSL
    mock_command "pkg-config" "exit 1"
    mock_command "dpkg" "echo 'dpkg version'"

    # Run: bootstrap.sh check
    run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Should provide installation command
    assert_failure
    assert_output_contains "sudo"
    assert_output_contains "install"
}

# Test: bootstrap.sh handles partial OpenSSL installation
test_bootstrap_handles_partial_openssl() {
    # Mock: OpenSSL library present but headers missing
    mock_command "pkg-config" "echo 'Package openssl was not found' && exit 1"

    # Run: bootstrap.sh check
    run bash "$BOOTSTRAP_SCRIPT" --check-only

    # Assert: Should detect incomplete installation
    assert_failure
    assert_output_contains "development"
}

# Test: bootstrap.sh dry-run mode doesn't modify system
test_bootstrap_dry_run_mode() {
    # Mock: Missing OpenSSL
    mock_command "pkg-config" "exit 1"
    mock_command "dpkg" "echo 'dpkg version'"

    # Run: bootstrap.sh with dry-run
    run bash "$BOOTSTRAP_SCRIPT" --dry-run

    # Assert: Should not call sudo
    assert_success
    assert_output_not_contains "Installing"
}

# Run all tests
run_test_suite "bootstrap.sh Unit Tests" \
    test_bootstrap_detects_missing_openssl \
    test_bootstrap_detects_fedora_package_name \
    test_bootstrap_detects_debian_package_name \
    test_bootstrap_installs_dependencies \
    test_bootstrap_builds_rustyclawd \
    test_bootstrap_verifies_cargo_binary \
    test_bootstrap_handles_build_failure \
    test_bootstrap_detects_missing_cargo \
    test_bootstrap_creates_artifacts_dir \
    test_bootstrap_writes_build_status \
    test_bootstrap_includes_openssl_version \
    test_bootstrap_provides_install_instructions \
    test_bootstrap_handles_partial_openssl \
    test_bootstrap_dry_run_mode

print_summary
