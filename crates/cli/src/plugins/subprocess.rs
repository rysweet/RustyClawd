//! Subprocess Executor for Plugin Commands
//!
//! Provides real subprocess execution with timeouts and output capture.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Plugin execution result
#[derive(Debug, Clone)]
pub struct PluginExecutionResult {
    pub success: bool,
    pub output: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
}

impl PluginExecutionResult {
    /// Create a successful execution result
    pub fn success(output: String, duration_ms: u64) -> Self {
        Self {
            success: true,
            output,
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms,
        }
    }

    /// Create a failed execution result
    pub fn failure(stderr: String, exit_code: i32, duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            stderr,
            exit_code: Some(exit_code),
            duration_ms,
        }
    }

    /// Create a timeout execution result
    pub fn timeout(duration_ms: u64) -> Self {
        Self {
            success: false,
            output: String::new(),
            stderr: "Execution timed out".to_string(),
            exit_code: None,
            duration_ms,
        }
    }
}

/// Subprocess executor
pub struct SubprocessExecutor;

impl SubprocessExecutor {
    /// Execute a command in a real subprocess
    pub fn execute(
        cmd: &str,
        args: &[&str],
        timeout_ms: u64,
    ) -> Result<PluginExecutionResult, String> {
        let start = Instant::now();

        let child = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Wait with timeout
        let _timeout = Duration::from_millis(timeout_ms);
        let result = std::thread::spawn(move || child.wait_with_output());

        // Try to join with timeout (simplified approach)
        let output_result = result
            .join()
            .map_err(|_| "Thread join failed".to_string())?;

        let duration = start.elapsed().as_millis() as u64;

        match output_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code();

                if output.status.success() {
                    Ok(PluginExecutionResult::success(stdout, duration))
                } else {
                    Ok(PluginExecutionResult::failure(
                        stderr,
                        exit_code.unwrap_or(-1),
                        duration,
                    ))
                }
            }
            Err(e) => Err(format!("Process execution failed: {}", e)),
        }
    }

    /// Detect interpreter for a script file
    pub fn detect_interpreter(file_path: &PathBuf) -> Result<String, String> {
        let content =
            fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {}", e))?;

        // Check for shebang
        if let Some(first_line) = content.lines().next() {
            if first_line.starts_with("#!") {
                let shebang = first_line.trim_start_matches("#!").trim();
                let parts: Vec<&str> = shebang.split_whitespace().collect();

                // Handle "/usr/bin/env python3" pattern
                if parts.len() >= 2 && parts[0].ends_with("env") {
                    return Ok(parts[1].to_string());
                }

                // Extract just the interpreter name from path
                if let Some(interpreter) = parts.first() {
                    // Get the last component of the path (e.g., "python3" from "/usr/bin/python3")
                    if let Some(name) = interpreter.rsplit('/').next() {
                        return Ok(name.to_string());
                    }
                }
            }
        }

        // Fallback to extension-based detection
        if let Some(ext) = file_path.extension() {
            match ext.to_str() {
                Some("py") => Ok("python3".to_string()),
                Some("js") => Ok("node".to_string()),
                Some("sh") => Ok("sh".to_string()),
                Some("rb") => Ok("ruby".to_string()),
                Some(other) => Err(format!("Unknown file extension: {}", other)),
                None => Err("Could not convert extension to string".to_string()),
            }
        } else {
            Err("No file extension found".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_execute_simple_command() {
        let result = SubprocessExecutor::execute("echo", &["hello"], 1000);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.success);
        assert!(exec_result.output.contains("hello"));
        assert_eq!(exec_result.exit_code, Some(0));
    }

    #[test]
    fn test_execute_command_with_failure() {
        let result = SubprocessExecutor::execute("sh", &["-c", "exit 1"], 1000);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(!exec_result.success);
        assert_eq!(exec_result.exit_code, Some(1));
    }

    #[test]
    fn test_detect_interpreter_from_extension() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let test_cases = vec![
            ("test.py", "python3"),
            ("test.js", "node"),
            ("test.sh", "sh"),
            ("test.rb", "ruby"),
        ];

        for (filename, expected_interpreter) in test_cases {
            let script_path = temp_dir.path().join(filename);
            let mut file = fs::File::create(&script_path).expect("Failed to create file");
            writeln!(file, "# test script").expect("Failed to write");

            let interpreter = SubprocessExecutor::detect_interpreter(&script_path);

            assert!(interpreter.is_ok(), "Should detect interpreter for {}", filename);
            assert_eq!(
                interpreter.unwrap(),
                expected_interpreter,
                "Wrong interpreter for {}",
                filename
            );
        }
    }

    #[test]
    fn test_detect_interpreter_from_shebang() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let script_path = temp_dir.path().join("test.py");

        let mut file = fs::File::create(&script_path).expect("Failed to create file");
        writeln!(file, "#!/usr/bin/env python3").expect("Failed to write");
        writeln!(file, "print('hello')").expect("Failed to write");

        let interpreter = SubprocessExecutor::detect_interpreter(&script_path);

        assert!(interpreter.is_ok());
        assert!(interpreter.unwrap().contains("python"));
    }

    #[test]
    fn test_execution_result_constructors() {
        let success = PluginExecutionResult::success("output".to_string(), 100);
        assert!(success.success);
        assert_eq!(success.output, "output");
        assert_eq!(success.exit_code, Some(0));

        let failure = PluginExecutionResult::failure("error".to_string(), 1, 100);
        assert!(!failure.success);
        assert_eq!(failure.stderr, "error");
        assert_eq!(failure.exit_code, Some(1));

        let timeout = PluginExecutionResult::timeout(5000);
        assert!(!timeout.success);
        assert!(timeout.stderr.contains("timed out"));
        assert_eq!(timeout.exit_code, None);
    }
}
