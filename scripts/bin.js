#!/usr/bin/env node
/**
 * NPX wrapper for RustyClawd
 * Executes the Rust binary with passed arguments
 */

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Find the Rust binary
const binaryPath = path.join(__dirname, 'target', 'release', 'claude');

// Check if binary exists
if (!fs.existsSync(binaryPath)) {
  console.error('RustyClawd binary not found. Building...');
  console.error('This may take a few minutes on first run.');
  console.error('Run: cargo build --release');
  process.exit(1);
}

// Spawn the Rust binary with all arguments
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env: process.env
});

// Forward exit code
child.on('exit', (code) => {
  process.exit(code || 0);
});

// Handle errors
child.on('error', (err) => {
  console.error('Failed to start RustyClawd:', err.message);
  process.exit(1);
});
