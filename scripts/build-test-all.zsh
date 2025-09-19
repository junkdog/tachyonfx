#!/usr/bin/env zsh

# build-test-all.zsh
# Performs all build/test steps from GitHub Actions main pipeline

set -e  # Exit on any error

echo "🏗 Starting complete build and test pipeline..."

# Set cargo color output
export CARGO_TERM_COLOR=always

# Function to run a command with status
run_step() {
    local step_name="$1"
    shift
    echo ""
    echo "📦 $step_name"
    echo "   Running: $@"

    # Capture output and exit code
    local output
    local exit_code
    output=$("$@" 2>&1)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo "✅ $step_name completed"
    else
        echo "❌ $step_name failed"
        echo "Command output:"
        echo "$output"
        exit $exit_code
    fi
}

echo ""
echo "🔧 Build & Test Matrix"
echo "======================"

# std variants with additional features
echo ""
echo "🚀 Testing std + sendable"
run_step "Build (std)" cargo build --verbose --features="std,crossterm,dsl"
run_step "Test (std)" cargo test --verbose --lib --features="std,crossterm,dsl"
run_step "Build (std + sendable)" cargo build --verbose --features="std,crossterm,dsl,sendable"
run_step "Test (std + sendable)" cargo test --verbose --lib --features="std,crossterm,dsl,sendable"

echo ""
echo "🚀 Testing std + std-duration"
run_step "Build (std)" cargo build --verbose --features="std,crossterm,dsl"
run_step "Test (std)" cargo test --verbose --lib --features="std,crossterm,dsl"
run_step "Build (std + std-duration)" cargo build --verbose --features="std,crossterm,dsl,std-duration"
run_step "Test (std + std-duration)" cargo test --verbose --lib --features="std,crossterm,dsl,std-duration"

echo ""
echo "🚀 Testing std + web-time"
run_step "Build (std)" cargo build --verbose --features="std,crossterm,dsl"
run_step "Test (std)" cargo test --verbose --lib --features="std,crossterm,dsl"
run_step "Build (std + web-time)" cargo build --verbose --features="std,crossterm,dsl,web-time"
run_step "Test (std + web-time)" cargo test --verbose --lib --features="std,crossterm,dsl,web-time"

echo ""
echo "🚀 Testing no-std + base"
run_step "Build (no-std)" cargo build --verbose --no-default-features
run_step "Test (no-std)" cargo test --verbose --lib --no-default-features
run_step "Build (no-std + base)" cargo build --verbose --no-default-features
run_step "Test (no-std + base)" cargo test --verbose --lib --no-default-features

echo ""
echo "🔍 Clippy Checks"
echo "================"
run_step "Clippy (all targets)" cargo clippy --all-targets

echo ""
echo "🔍 Additional Clippy Checks (with feature variations)"
echo "===================================================="
run_step "Clippy (std features)" cargo clippy --all-targets --features="std,crossterm,dsl"
run_step "Clippy (sendable)" cargo clippy --all-targets --features="std,crossterm,dsl,sendable"
run_step "Clippy (std-duration)" cargo clippy --all-targets --features="std,crossterm,dsl,std-duration"
run_step "Clippy (web-time)" cargo clippy --all-targets --features="std,crossterm,dsl,web-time"
run_step "Clippy (no-std)" cargo clippy --all-targets --no-default-features

echo ""
echo "🔍 Clippy with warnings as errors (like pre-commit hook)"
echo "======================================================="
run_step "Clippy (warnings as errors)" cargo clippy --all-targets -- -D warnings

echo ""
echo "🎨 Code Formatting"
echo "=================="
run_step "Format code (nightly)" cargo +nightly fmt --check

echo ""
echo "🎉 All build and test steps completed successfully!"
echo "   Pipeline matches GitHub Actions workflow + additional clippy checks + formatting"