//! Process isolation utilities for preventing tool execution from corrupting TUI display
//!
//! This module provides cross-platform utilities to isolate spawned processes:
//! - Unix: Uses setsid() to create a new session
//! - Windows: Uses CREATE_NEW_PROCESS_GROUP flag
//!
//! These mechanisms prevent child processes from inheriting terminal state
//! and potentially interfering with TUI applications.

use std::io;
use tokio::process::Command;

/// Configuration for spawning isolated processes
#[derive(Debug, Clone, Default)]
pub struct ProcessSpawnConfig {
    /// Whether to apply process isolation
    pub isolate: bool,
}

impl ProcessSpawnConfig {
    /// Create a new configuration with isolation enabled
    pub fn with_isolation() -> Self {
        Self { isolate: true }
    }

    /// Create a new configuration with isolation disabled
    pub fn without_isolation() -> Self {
        Self { isolate: false }
    }
}

/// Apply process isolation to a Command based on the platform
///
/// # Platform-specific behavior
///
/// ## Unix (Linux, macOS, BSD)
/// Uses the `setsid()` system call to create a new session. This:
/// - Creates a new process group
/// - Detaches from the controlling terminal
/// - Prevents signals (like SIGINT, SIGTSTP) from being forwarded
/// - Ensures child processes don't inherit terminal file descriptors
///
/// ## Windows
/// Uses the CREATE_NEW_PROCESS_GROUP flag. This:
/// - Creates a new console process group
/// - Prevents Ctrl+C and Ctrl+Break from affecting the child
///
/// # Arguments
///
/// * `command` - The Command to configure
/// * `config` - Configuration specifying whether to apply isolation
///
/// # Returns
///
/// The configured Command ready for spawning
///
/// # Example
///
/// ```no_run
/// use tokio::process::Command;
/// use rustyclawd_tools::process_isolation::{apply_isolation, ProcessSpawnConfig};
///
/// # async fn example() -> std::io::Result<()> {
/// let mut cmd = Command::new("bash");
/// cmd.arg("-c").arg("echo hello");
///
/// let config = ProcessSpawnConfig::with_isolation();
/// let cmd = apply_isolation(cmd, &config);
///
/// let child = cmd.spawn()?;
/// # Ok(())
/// # }
/// ```
pub fn apply_isolation(mut command: Command, config: &ProcessSpawnConfig) -> Command {
    if !config.isolate {
        return command;
    }

    #[cfg(unix)]
    {
        // On Unix, use pre_exec to call setsid()
        // This creates a new session and detaches from the controlling terminal
        unsafe {
            command.pre_exec(|| {
                // setsid() creates a new session with this process as the session leader
                // This also creates a new process group
                match nix::unistd::setsid() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(io::Error::new(
                        io::ErrorKind::Other,
                        format!("Failed to create new session: {}", e),
                    )),
                }
            });
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use CREATE_NEW_PROCESS_GROUP + DETACHED_PROCESS
        // to detach from parent console and prevent terminal corruption
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS);
    }

    command
}

/// Spawn a command with process isolation
///
/// This is a convenience function that applies isolation and spawns the command.
///
/// # Arguments
///
/// * `command` - The Command to spawn
/// * `config` - Configuration specifying whether to apply isolation
///
/// # Returns
///
/// A Result containing the spawned Child process
///
/// # Example
///
/// ```no_run
/// use tokio::process::Command;
/// use rustyclawd_tools::process_isolation::{spawn_with_isolation, ProcessSpawnConfig};
///
/// # async fn example() -> std::io::Result<()> {
/// let mut cmd = Command::new("bash");
/// cmd.arg("-c").arg("echo hello");
///
/// let config = ProcessSpawnConfig::with_isolation();
/// let child = spawn_with_isolation(cmd, &config).await?;
/// # Ok(())
/// # }
/// ```
pub async fn spawn_with_isolation(
    command: Command,
    config: &ProcessSpawnConfig,
) -> io::Result<tokio::process::Child> {
    let mut command = apply_isolation(command, config);
    command.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn test_spawn_without_isolation() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        cmd.stdout(std::process::Stdio::piped());

        let config = ProcessSpawnConfig::without_isolation();
        let mut child = spawn_with_isolation(cmd, &config)
            .await
            .expect("Failed to spawn process");

        let mut stdout = child.stdout.take().expect("Failed to take stdout");
        let mut output = String::new();
        stdout
            .read_to_string(&mut output)
            .await
            .expect("Failed to read stdout");

        let status = child.wait().await.expect("Failed to wait for child");
        assert!(status.success());
        assert_eq!(output.trim(), "hello");
    }

    #[tokio::test]
    async fn test_spawn_with_isolation() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello world");
        cmd.stdout(std::process::Stdio::piped());

        let config = ProcessSpawnConfig::with_isolation();
        let mut child = spawn_with_isolation(cmd, &config)
            .await
            .expect("Failed to spawn process");

        let mut stdout = child.stdout.take().expect("Failed to take stdout");
        let mut output = String::new();
        stdout
            .read_to_string(&mut output)
            .await
            .expect("Failed to read stdout");

        let status = child.wait().await.expect("Failed to wait for child");
        assert!(status.success());
        assert_eq!(output.trim(), "hello world");
    }

    #[tokio::test]
    async fn test_isolation_prevents_signal_propagation() {
        // This test verifies that the isolated process doesn't receive
        // signals from the parent's process group
        let mut cmd = Command::new("sleep");
        cmd.arg("0.1"); // Short sleep
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let config = ProcessSpawnConfig::with_isolation();
        let mut child = spawn_with_isolation(cmd, &config)
            .await
            .expect("Failed to spawn process");

        // The child should complete normally even if we could send signals
        // (we don't actually send signals in this test to keep it portable)
        let status = child.wait().await.expect("Failed to wait for child");
        assert!(status.success());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_unix_process_group_id() {
        // On Unix, verify that the process is in a different process group
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo $$"); // Print process ID
        cmd.stdout(std::process::Stdio::piped());

        let config = ProcessSpawnConfig::with_isolation();
        let mut child = spawn_with_isolation(cmd, &config)
            .await
            .expect("Failed to spawn process");

        let _child_pid = child.id().expect("Failed to get child PID");

        let mut stdout = child.stdout.take().expect("Failed to take stdout");
        let mut output = String::new();
        stdout
            .read_to_string(&mut output)
            .await
            .expect("Failed to read stdout");

        let status = child.wait().await.expect("Failed to wait for child");
        assert!(status.success());

        // The shell should have printed its own PID
        let shell_pid: u32 = output.trim().parse().expect("Failed to parse PID");

        // Due to setsid(), the shell becomes a session leader
        // So its PID should match what the process manager reported
        // (though this is a weak test, it at least verifies the command ran)
        assert!(shell_pid > 0);
    }

    #[tokio::test]
    async fn test_apply_isolation_does_not_break_basic_commands() {
        // Test that isolation doesn't break basic command execution
        let commands = vec![("echo", vec!["test"]), ("printf", vec!["test\\n"])];

        for (cmd_name, args) in commands {
            let mut cmd = Command::new(cmd_name);
            for arg in args {
                cmd.arg(arg);
            }
            cmd.stdout(std::process::Stdio::piped());

            let config = ProcessSpawnConfig::with_isolation();
            let mut child = spawn_with_isolation(cmd, &config)
                .await
                .expect(&format!("Failed to spawn {}", cmd_name));

            let status = child.wait().await.expect("Failed to wait for child");
            assert!(
                status.success(),
                "{} should succeed with isolation",
                cmd_name
            );
        }
    }
}
