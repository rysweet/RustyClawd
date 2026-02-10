#!/usr/bin/env rust-script
//! Fetch Claude Code features from official sources
//!
//! This script uses WebFetch to get Claude Code release notes and documentation
//! while respecting trademark fair use (descriptive references only).

use std::process::Command;

fn main() {
    println!("🔍 Fetching latest Claude Code features (respecting trademark fair use)...\n");

    // Fetch CHANGELOG.md from official Claude Code repository (descriptive reference for compatibility)
    let changelog_url = "https://raw.githubusercontent.com/anthropics/claude-code/main/CHANGELOG.md";
    let readme_url = "https://raw.githubusercontent.com/anthropics/claude-code/main/README.md";

    println!("📄 Fetching CHANGELOG.md...");
    fetch_and_display(changelog_url, "Extract all feature announcements, new tools, and capabilities mentioned");

    println!("\n📄 Fetching README.md...");
    fetch_and_display(readme_url, "List all tools, features, and capabilities with their descriptions");
}

fn fetch_and_display(url: &str, prompt: &str) {
    let output = Command::new("curl")
        .arg("-s")
        .arg(url)
        .output()
        .expect("Failed to fetch URL");

    if output.status.success() {
        let content = String::from_utf8_lossy(&output.stdout);
        println!("✅ Fetched {} bytes", content.len());

        // Print first 1000 chars as preview
        let preview: String = content.chars().take(1000).collect();
        println!("Preview:\n{}\n...", preview);
    } else {
        eprintln!("❌ Failed to fetch {}", url);
    }
}
