//! Claude Code Sync Monitor CLI
//!
//! Command-line interface for running the sync monitor.
//!
//! Usage:
//!   cargo run --package rustyclawd-tools --example claude_code_sync_cli -- \
//!     --inventory .claude/data/feature_inventory.yaml \
//!     --ledger .claude/data/sync_ledger.json \
//!     --token $GITHUB_TOKEN \
//!     --repo owner/repo

use rustyclawd_tools::claude_code_sync::SyncMonitor;
use std::env;
use std::process;

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    let mut inventory_path = String::from(".claude/data/feature_inventory.yaml");
    let mut ledger_path = String::from(".claude/data/sync_ledger.json");
    let mut github_token = String::new();
    let mut repo = String::new();

    // Simple argument parsing
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--inventory" => {
                i += 1;
                if i < args.len() {
                    inventory_path = args[i].clone();
                }
            }
            "--ledger" => {
                i += 1;
                if i < args.len() {
                    ledger_path = args[i].clone();
                }
            }
            "--token" => {
                i += 1;
                if i < args.len() {
                    github_token = args[i].clone();
                }
            }
            "--repo" => {
                i += 1;
                if i < args.len() {
                    repo = args[i].clone();
                }
            }
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                process::exit(1);
            }
        }
        i += 1;
    }

    // Validate required arguments
    if github_token.is_empty() {
        eprintln!("Error: --token is required");
        print_usage();
        process::exit(1);
    }

    if repo.is_empty() {
        eprintln!("Error: --repo is required (format: owner/repo)");
        print_usage();
        process::exit(1);
    }

    println!("Claude Code Sync Monitor");
    println!("========================");
    println!();
    println!("Inventory: {}", inventory_path);
    println!("Ledger:    {}", ledger_path);
    println!("Repo:      {}", repo);
    println!();

    // Create and run sync monitor
    let mut monitor = SyncMonitor::new(inventory_path, ledger_path, github_token, repo);

    match monitor.run().await {
        Ok(report) => {
            println!("Sync completed successfully!");
            println!();
            println!("Results:");
            println!("  Claude Code features found: {}", report.claude_features_found);
            println!("  Gaps identified:            {}", report.gaps_identified);
            println!("  Issues created:             {}", report.issues_created);
            println!();

            if !report.issues.is_empty() {
                println!("Created issues:");
                for issue in &report.issues {
                    println!("  #{}: {} - {}", issue.number, issue.title, issue.url);
                }
            } else {
                println!("No new issues created (all gaps already tracked).");
            }

            process::exit(0);
        }
        Err(e) => {
            eprintln!("Error running sync monitor: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Usage: claude_code_sync_cli [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --inventory <path>  Path to feature inventory YAML (default: .claude/data/feature_inventory.yaml)");
    println!("  --ledger <path>     Path to issue ledger JSON (default: .claude/data/sync_ledger.json)");
    println!("  --token <token>     GitHub API token (required)");
    println!("  --repo <owner/repo> GitHub repository (required)");
    println!("  --help, -h          Show this help message");
    println!();
    println!("Example:");
    println!("  cargo run --package rustyclawd-tools --example claude_code_sync_cli -- \\");
    println!("    --token $GITHUB_TOKEN \\");
    println!("    --repo myorg/myrepo");
}
