#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! serde = { version = "1.0", features = ["derive"] }
//! serde_yaml = "0.9"
//! chrono = "0.4"
//! colored = "2.0"
//! ```

use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::time::{Duration, Instant};
use std::path::PathBuf;
use std::fs;
use colored::*;
use chrono::Local;

/// Configuration for continuous testing
#[derive(Debug)]
struct TestConfig {
    max_iterations: Option<usize>,
    delay_between_runs: Duration,
    rusty_binary: PathBuf,
    test_scenario: PathBuf,
    output_dir: PathBuf,
}

impl TestConfig {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();
        
        let max_iterations = args.iter()
            .position(|a| a == "--max-iterations")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok());
        
        let delay_secs = args.iter()
            .position(|a| a == "--delay")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        
        let rusty_binary = args.iter()
            .position(|a| a == "--binary")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("rusty"));
        
        let test_scenario = args.iter()
            .position(|a| a == "--scenario")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("test-all-features"));
        
        let output_dir = args.iter()
            .position(|a| a == "--output")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp/rustyclawd-continuous-tests"));

        TestConfig {
            max_iterations,
            delay_between_runs: Duration::from_secs(delay_secs),
            rusty_binary,
            test_scenario,
            output_dir,
        }
    }
}

/// Result of a single test run
#[derive(Debug)]
struct TestResult {
    iteration: usize,
    timestamp: String,
    duration: Duration,
    success: bool,
    output_file: PathBuf,
    error: Option<String>,
}

/// Main continuous tester
struct ContinuousTester {
    config: TestConfig,
    results: Vec<TestResult>,
    start_time: Instant,
}

impl ContinuousTester {
    fn new(config: TestConfig) -> Self {
        // Ensure output directory exists
        fs::create_dir_all(&config.output_dir)
            .expect("Failed to create output directory");
        
        ContinuousTester {
            config,
            results: Vec::new(),
            start_time: Instant::now(),
        }
    }

    fn run(&mut self) {
        println!("{}", "🦀 RustyClawd Continuous Tester".bright_cyan().bold());
        println!("{}", "================================".bright_cyan());
        println!("Binary: {}", self.config.rusty_binary.display());
        println!("Scenario: {}", self.config.test_scenario.display());
        println!("Output: {}", self.config.output_dir.display());
        
        if let Some(max) = self.config.max_iterations {
            println!("Max iterations: {}", max);
        } else {
            println!("Max iterations: {}", "unlimited".yellow());
        }
        
        println!("Delay: {:?}\n", self.config.delay_between_runs);
        println!("{}", "Press Ctrl+C to stop\n".dimmed());

        let mut iteration = 1;
        
        loop {
            // Check if we should stop
            if let Some(max) = self.config.max_iterations {
                if iteration > max {
                    break;
                }
            }

            // Run test
            println!("{} Iteration #{}", "▶".green().bold(), iteration);
            let result = self.run_test_iteration(iteration);
            
            // Display result
            self.display_result(&result);
            self.results.push(result);
            
            // Check if we should continue
            if let Some(max) = self.config.max_iterations {
                if iteration >= max {
                    break;
                }
            }

            // Wait before next iteration
            println!("\n{} Waiting {:?} before next run...\n", 
                     "⏳".yellow(), self.config.delay_between_runs);
            std::thread::sleep(self.config.delay_between_runs);
            
            iteration += 1;
        }

        // Print final summary
        self.print_summary();
    }

    fn run_test_iteration(&self, iteration: usize) -> TestResult {
        let start = Instant::now();
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let output_file = self.config.output_dir.join(format!("run_{:04}_{}.log", iteration, timestamp));

        // Prepare the prompt
        let prompt = format!(
            "Run comprehensive E2E tests using the test scenario '{}'. \
             Report all results, file issues for any bugs found, and fix critical bugs immediately.",
            self.config.test_scenario.display()
        );

        println!("  {} Launching rusty subprocess...", "→".dimmed());
        
        // Run rusty in subprocess with depth limit
        let mut child = match Command::new(&self.config.rusty_binary)
            .arg("--mode").arg("cli")
            .arg("--max-depth").arg("1")  // Prevent infinite recursion
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                return TestResult {
                    iteration,
                    timestamp,
                    duration: start.elapsed(),
                    success: false,
                    output_file,
                    error: Some(format!("Failed to spawn process: {}", e)),
                };
            }
        };

        // Send the prompt
        if let Some(mut stdin) = child.stdin.take() {
            let _ = writeln!(stdin, "{}", prompt);
            drop(stdin); // Close stdin to signal we're done
        }

        // Capture output
        let mut output_content = String::new();
        
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                println!("    {}", line.dimmed());
                output_content.push_str(&line);
                output_content.push('\n');
            }
        }

        // Wait for completion (with timeout)
        let success = match wait_with_timeout(&mut child, Duration::from_secs(600)) {
            Ok(status) => {
                output_content.push_str(&format!("\n\nExit status: {:?}\n", status));
                status.success()
            }
            Err(e) => {
                let _ = child.kill();
                output_content.push_str(&format!("\n\nTimeout or error: {}\n", e));
                false
            }
        };

        // Save output
        let _ = fs::write(&output_file, &output_content);

        TestResult {
            iteration,
            timestamp,
            duration: start.elapsed(),
            success,
            output_file,
            error: if success { None } else { Some("Test failed".to_string()) },
        }
    }

    fn display_result(&self, result: &TestResult) {
        let status_icon = if result.success { "✅" } else { "❌" };
        let status_text = if result.success { 
            "PASSED".green().bold() 
        } else { 
            "FAILED".red().bold() 
        };

        println!("  {} Test {} in {:.2?}", 
                 status_icon, status_text, result.duration);
        println!("  {} Log: {}", "📄".dimmed(), result.output_file.display());
        
        if let Some(ref error) = result.error {
            println!("  {} Error: {}", "⚠".red(), error);
        }
    }

    fn print_summary(&self) {
        let total_duration = self.start_time.elapsed();
        let total = self.results.len();
        let passed = self.results.iter().filter(|r| r.success).count();
        let failed = total - passed;
        let success_rate = if total > 0 { 
            (passed as f64 / total as f64) * 100.0 
        } else { 
            0.0 
        };

        println!("\n{}", "═══════════════════════════════════════".bright_cyan());
        println!("{}", "📊 CONTINUOUS TESTING SUMMARY".bright_cyan().bold());
        println!("{}", "═══════════════════════════════════════".bright_cyan());
        println!("\n{} Duration: {:.2?}", "⏱", total_duration);
        println!("{} Total runs: {}", "🔢", total);
        println!("{} Passed: {}", "✅", passed.to_string().green());
        println!("{} Failed: {}", "❌", failed.to_string().red());
        println!("{} Success rate: {:.1}%", "📈", success_rate);
        
        println!("\n{}", "Output files:".bold());
        for result in &self.results {
            let icon = if result.success { "✅" } else { "❌" };
            println!("  {} Run #{:04}: {}", icon, result.iteration, result.output_file.display());
        }
        
        println!("\n{}", "Testing complete! 🦀".bright_cyan().bold());
    }
}

fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> std::io::Result<std::process::ExitStatus> {
    let start = Instant::now();
    
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None => {
                if start.elapsed() > timeout {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "Process timed out"
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

fn print_usage() {
    println!("Usage: continuous_tester [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --max-iterations N    Run N iterations then stop (default: unlimited)");
    println!("  --delay SECS         Wait SECS seconds between runs (default: 30)");
    println!("  --binary PATH        Path to rusty binary (default: 'rusty')");
    println!("  --scenario NAME      Test scenario to run (default: 'test-all-features')");
    println!("  --output DIR         Output directory for logs (default: /tmp/rustyclawd-continuous-tests)");
    println!("  --help               Show this help message");
    println!();
    println!("Example:");
    println!("  ./continuous_tester --max-iterations 5 --delay 60");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    let config = TestConfig::from_args();
    let mut tester = ContinuousTester::new(config);
    tester.run();
}
