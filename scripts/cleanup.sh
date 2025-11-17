#!/bin/bash
# Rust build artifact cleanup script
# Prevents target/ directory from filling disk

set -e

echo "🧹 Cleaning Rust build artifacts..."

# Check if in Rust project
if [ ! -f "Cargo.toml" ]; then
    echo "❌ Error: Not in a Rust project directory"
    echo "   Run this script from the project root containing Cargo.toml"
    exit 1
fi

# Show current disk usage
echo ""
echo "📊 Current disk usage:"
df -h . | tail -1

# Show target directory size if it exists
if [ -d "target" ]; then
    echo ""
    echo "📁 Target directory size before cleanup:"
    du -sh target/ 2>/dev/null || echo "   (cannot measure)"
fi

# Option 1: Clean all artifacts (most aggressive)
if [ "$1" = "--all" ]; then
    echo ""
    echo "🗑️  Cleaning ALL build artifacts..."
    cargo clean
    echo "✅ All artifacts cleaned"

# Option 2: Clean old artifacts only (recommended)
elif command -v cargo-sweep &> /dev/null; then
    echo ""
    echo "🧹 Cleaning artifacts older than 7 days with cargo-sweep..."
    cargo sweep -t 7
    echo "✅ Old artifacts cleaned (kept last 7 days)"

# Option 3: Clean debug only (keeps release)
elif [ "$1" = "--debug" ]; then
    echo ""
    echo "🗑️  Cleaning debug artifacts only..."
    find target -type d -name "debug" -exec rm -rf {} + 2>/dev/null || true
    echo "✅ Debug artifacts cleaned (release preserved)"

# Default: Clean everything except release
else
    echo ""
    echo "🗑️  Cleaning debug and test artifacts (preserving release)..."
    cargo clean --doc
    find target -type d -name "debug" -exec rm -rf {} + 2>/dev/null || true
    find target -type d -name ".fingerprint" -exec rm -rf {} + 2>/dev/null || true
    echo "✅ Debug artifacts cleaned"

    echo ""
    echo "💡 Tip: For more aggressive cleanup:"
    echo "   ./scripts/cleanup.sh --all          # Remove everything"
    echo "   ./scripts/cleanup.sh --debug        # Remove debug only"
    echo "   cargo install cargo-sweep           # Install sweep tool"
    echo "   ./scripts/cleanup.sh                # Use sweep (keeps 7 days)"
fi

# Show final disk usage
echo ""
echo "📊 Disk usage after cleanup:"
df -h . | tail -1

if [ -d "target" ]; then
    echo ""
    echo "📁 Target directory size after cleanup:"
    du -sh target/ 2>/dev/null || echo "   (empty or removed)"
fi

echo ""
echo "✨ Cleanup complete!"
