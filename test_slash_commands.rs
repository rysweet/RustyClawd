#!/usr/bin/env rust-script
//! Test slash command discovery
//! ```cargo
//! [dependencies]
//! ```

use std::process::Command;

fn main() {
    println!("Testing slash command discovery...\n");

    // Build the test command
    let output = Command::new("cargo")
        .args(&["run", "--bin", "rustyclawd", "--", "--help"])
        .current_dir("/Users/ryan/src/declawed/claude-code-rs")
        .output()
        .expect("Failed to execute command");

    println!("Exit status: {}", output.status);
    println!("\nStdout:");
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("\nStderr:");
    println!("{}", String::from_utf8_lossy(&output.stderr));
}
