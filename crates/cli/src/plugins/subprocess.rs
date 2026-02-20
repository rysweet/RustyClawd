//! Subprocess Executor for Plugin Commands
//!
//! Provides real subprocess execution with timeouts and output capture.
//! Includes proper timeout enforcement with process tree killing.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
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

/// Kill a process and its children
#[cfg(unix)]
fn kill_process(child: &mut Child) -> Result<(), String> {
    let pid = child.id() as i32;

    // First try SIGTERM to the process group (negative PID kills the group)
    // This allows processes to clean up gracefully
    unsafe {
        libc::kill(-pid, libc::SIGTERM);
    }

    // Give the process a moment to terminate gracefully
    std::thread::sleep(Duration::from_millis(100));

    // Check if still running
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()), // Process has terminated
        Ok(None) => {
            // Still running, send SIGKILL
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
            // Wait for it to actually die
            let _ = child.wait();
        }
        Err(e) => return Err(format!("Failed to check process status: {}", e)),
    }

    Ok(())
}

/// Kill a process on Windows
#[cfg(windows)]
fn kill_process(child: &mut Child) -> Result<(), String> {
    child
        .kill()
        .map_err(|e| format!("Failed to kill process: {}", e))?;
    let _ = child.wait();
    Ok(())
}

/// Subprocess executor
pub struct SubprocessExecutor;

impl SubprocessExecutor {
    /// Execute a command in a real subprocess with enforced timeout
    pub fn execute(
        cmd: &str,
        args: &[&str],
        timeout_ms: u64,
    ) -> Result<PluginExecutionResult, String> {
        let start = Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Build the command
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // On Unix, create a new process group so we can kill all children
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            // SAFETY: setpgid is safe to call in pre_exec
            unsafe {
                command.pre_exec(|| {
                    // Create a new process group with this process as leader
                    libc::setpgid(0, 0);
                    Ok(())
                });
            }
        }

        let mut child = command
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Poll for completion with timeout
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process completed
                    let duration = start.elapsed().as_millis() as u64;

                    // Read output from the completed process
                    let stdout = child
                        .stdout
                        .take()
                        .map(|mut s| {
                            let mut buf = Vec::new();
                            std::io::Read::read_to_end(&mut s, &mut buf).ok();
                            String::from_utf8_lossy(&buf).to_string()
                        })
                        .unwrap_or_default();

                    let stderr = child
                        .stderr
                        .take()
                        .map(|mut s| {
                            let mut buf = Vec::new();
                            std::io::Read::read_to_end(&mut s, &mut buf).ok();
                            String::from_utf8_lossy(&buf).to_string()
                        })
                        .unwrap_or_default();

                    if status.success() {
                        return Ok(PluginExecutionResult::success(stdout, duration));
                    } else {
                        return Ok(PluginExecutionResult::failure(
                            stderr,
                            status.code().unwrap_or(-1),
                            duration,
                        ));
                    }
                }
                Ok(None) => {
                    // Process still running, check timeout
                    if start.elapsed() >= timeout {
                        // Timeout exceeded - kill the process
                        let duration = start.elapsed().as_millis() as u64;
                        kill_process(&mut child)?;
                        return Ok(PluginExecutionResult::timeout(duration));
                    }
                    // Sleep briefly before checking again
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    return Err(format!("Failed to check process status: {}", e));
                }
            }
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
    fn test_timeout_enforced() {
        // Start a process that sleeps for 5 seconds, but only give it 200ms
        let result = SubprocessExecutor::execute("sleep", &["5"], 200);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(!exec_result.success);
        assert!(exec_result.stderr.contains("timed out"));
        assert_eq!(exec_result.exit_code, None);
        // Should complete in roughly 200ms, not 5 seconds
        assert!(exec_result.duration_ms < 1000);
    }

    #[test]
    fn test_command_completes_before_timeout() {
        // Fast command with long timeout
        let result = SubprocessExecutor::execute("echo", &["fast"], 5000);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(exec_result.success);
        assert!(exec_result.output.contains("fast"));
        // Should complete quickly, not wait for timeout
        assert!(exec_result.duration_ms < 1000);
    }

    #[test]
    fn test_timeout_kills_process_tree() {
        // Start a shell that spawns a child process
        // The parent sleeps, and we need to ensure both are killed
        let result = SubprocessExecutor::execute("sh", &["-c", "sleep 10 & sleep 10"], 200);

        assert!(result.is_ok());
        let exec_result = result.unwrap();
        assert!(!exec_result.success);
        assert!(exec_result.stderr.contains("timed out"));
        // Should timeout quickly, not wait 10 seconds
        assert!(exec_result.duration_ms < 1000);
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

            assert!(
                interpreter.is_ok(),
                "Should detect interpreter for {}",
                filename
            );
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
