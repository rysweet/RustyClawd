#!/usr/bin/env node
/**
 * Post-install script for RustyClawd
 * Compiles the Rust binary after npm install
 */

const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

console.log('Building RustyClawd...');
console.log('This may take 1-2 minutes on first install.');

try {
  // Check if cargo is available
  execSync('cargo --version', { stdio: 'ignore' });
} catch (e) {
  console.error('\nError: Cargo (Rust toolchain) not found!');
  console.error('Install Rust from: https://rustup.rs');
  console.error('\nOr use pre-built binary installation instead.');
  process.exit(1);
}

try {
  // Build release binary
  // Use parent directory since script is in scripts/ subdirectory
  execSync('cargo build --release', {
    stdio: 'inherit',
    cwd: path.join(__dirname, '..')
  });

  console.log('\n✅ RustyClawd built successfully!');
  console.log('You can now use: npx github:rysweet/RustyClawd "your prompt"');
} catch (e) {
  console.error('\n❌ Build failed:', e.message);
  process.exit(1);
}
