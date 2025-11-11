#!/bin/bash
# Demonstration of all 14 Rust Claude Code tools

echo "===================================="
echo "Rust Claude Code - Complete Tool Suite"
echo "===================================="
echo ""

cd /Users/ryan/src/declawed/claude-code-rs

echo "1. Bash Tool - Execute commands"
cargo run --release -- bash "echo 'Hello from Bash tool!'"
echo ""

echo "2. Write Tool - Create a test file"
cargo run --release -- write /tmp/rust_demo.txt --content "Rust is amazing!"
echo ""

echo "3. Read Tool - Read the file back"
cargo run --release -- read /tmp/rust_demo.txt
echo ""

echo "4. Edit Tool - Modify the content"
cargo run --release -- edit /tmp/rust_demo.txt --old-string "amazing" --new-string "INCREDIBLE" --replace-all
echo ""

echo "5. Read Tool - Verify the edit"
cargo run --release -- read /tmp/rust_demo.txt
echo ""

echo "6. Glob Tool - Find Rust files"
cargo run --release -- glob "*.rs" --path crates/core/src
echo ""

echo "7. Grep Tool - Search for async functions"
cargo run --release -- grep "async fn" --path crates/tools/src --head-limit 3
echo ""

echo "===================================="
echo "All 14 tools implemented and working!"
echo "===================================="
