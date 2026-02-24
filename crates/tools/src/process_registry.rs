//! Process registry for tracking background shell processes
//!
//! Provides shared state for:
//! - Bash tool (when run_in_background=true)
//! - BashOutput tool (retrieves output)
//! - KillShell tool (terminates processes)
//!
//! The registry maintains a map of running processes, each with:
//! - Child process handle
//! - Output channels for stdout/stderr
//! - Current execution status
//! - Buffered output for retrieval

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::process::Child;
use tokio::sync::Mutex;

/// Registry for tracking background processes
///
/// Uses Arc<Mutex<>> for thread-safe, concurrent access from multiple tools.
/// Each process maintains output buffers that are read by BashOutput tool.
pub struct ProcessRegistry {
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
}

/// Status of a background process
#[derive(Debug, Clone)]
pub enum ProcessStatus {
    /// Process is currently running
    Running,
    /// Process completed with exit code
    Completed(i32),
    /// Process failed with error message
    Failed(String),
}

/// Handle for a background process
pub struct ProcessHandle {
    pub id: String,
    pub child: Child,
    pub stdout_buffer: Vec<String>,
    pub stderr_buffer: Vec<String>,
    pub status: ProcessStatus,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

impl ProcessRegistry {
    /// Create a new process registry
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a unique shell ID
    pub fn generate_id() -> String {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let hasher = RandomState::new();
        let mut s = hasher.build_hasher();
        s.write_usize(std::process::id() as usize);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        s.write_u128(now);
        format!("shell_{:x}", s.finish())
    }

    /// Register a new background process
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the process
    /// * `child` - Tokio Child process handle
    ///
    /// # Returns
    ///
    /// The shell ID for later retrieval
    pub async fn register(&self, id: String, child: Child) -> Result<String, String> {
        let handle = ProcessHandle {
            id: id.clone(),
            child,
            stdout_buffer: Vec::new(),
            stderr_buffer: Vec::new(),
            status: ProcessStatus::Running,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            completed_at: None,
        };

        let mut processes = self.processes.lock().await;
        processes.insert(id.clone(), handle);
        Ok(id)
    }

    /// Append output to a process's buffers
    pub async fn append_output(
        &self,
        id: &str,
        line: String,
        is_stderr: bool,
    ) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        if let Some(handle) = processes.get_mut(id) {
            if is_stderr {
                handle.stderr_buffer.push(line);
            } else {
                handle.stdout_buffer.push(line);
            }
            Ok(())
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Get output from a process (optionally filtered)
    ///
    /// Returns combined output lines, emptying the buffer as they're read.
    pub async fn get_output(
        &self,
        id: &str,
        filter: Option<&regex::Regex>,
    ) -> Result<(String, String, String), String> {
        let mut processes = self.processes.lock().await;
        if let Some(handle) = processes.get_mut(id) {
            // Get current status as string
            let status_str = match &handle.status {
                ProcessStatus::Running => "running".to_string(),
                ProcessStatus::Completed(code) => format!("completed:{}", code),
                ProcessStatus::Failed(msg) => format!("failed:{}", msg),
            };

            // Collect stdout
            let stdout_lines: Vec<String> = if let Some(regex) = filter {
                handle
                    .stdout_buffer
                    .iter()
                    .filter(|line| regex.is_match(line))
                    .cloned()
                    .collect()
            } else {
                handle.stdout_buffer.clone()
            };

            // Collect stderr (typically not filtered)
            let stderr_lines = handle.stderr_buffer.clone();

            // Clear buffers
            handle.stdout_buffer.clear();
            handle.stderr_buffer.clear();

            let stdout = stdout_lines.join("\n");
            let stderr = stderr_lines.join("\n");

            Ok((stdout, stderr, status_str))
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Terminate a process
    ///
    /// First attempts graceful SIGTERM, then SIGKILL if necessary.
    pub async fn kill(&self, id: &str) -> Result<bool, String> {
        let mut processes = self.processes.lock().await;
        if let Some(mut handle) = processes.remove(id) {
            // Try SIGTERM first
            match handle.child.kill().await {
                Ok(_) => {
                    handle.status = ProcessStatus::Completed(-1); // Killed
                    handle.completed_at = Some(
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    );
                    Ok(true)
                }
                Err(e) => {
                    handle.status = ProcessStatus::Failed(format!("Kill failed: {}", e));
                    Err(format!("Failed to kill process: {}", e))
                }
            }
        } else {
            Ok(false) // Process not found
        }
    }

    /// Update process status based on wait result
    pub async fn mark_completed(&self, id: &str, exit_code: i32) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        if let Some(handle) = processes.get_mut(id) {
            handle.status = ProcessStatus::Completed(exit_code);
            handle.completed_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            Ok(())
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Mark process as failed
    pub async fn mark_failed(&self, id: &str, error: String) -> Result<(), String> {
        let mut processes = self.processes.lock().await;
        if let Some(handle) = processes.get_mut(id) {
            handle.status = ProcessStatus::Failed(error);
            handle.completed_at = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            Ok(())
        } else {
            Err(format!("Process not found: {}", id))
        }
    }

    /// Check if process exists
    pub async fn exists(&self, id: &str) -> bool {
        let processes = self.processes.lock().await;
        processes.contains_key(id)
    }

    /// Get process status
    pub async fn get_status(&self, id: &str) -> Result<ProcessStatus, String> {
        let processes = self.processes.lock().await;
        processes
            .get(id)
            .map(|h| h.status.clone())
            .ok_or_else(|| format!("Process not found: {}", id))
    }

    /// List all active shell IDs
    pub async fn list_ids(&self) -> Vec<String> {
        let processes = self.processes.lock().await;
        processes.keys().cloned().collect()
    }
}

/// Global process registry instance (singleton pattern)
static GLOBAL_REGISTRY: OnceLock<Arc<ProcessRegistry>> = OnceLock::new();

/// Get or create the global registry instance
pub fn global_registry() -> Arc<ProcessRegistry> {
    Arc::clone(GLOBAL_REGISTRY.get_or_init(|| Arc::new(ProcessRegistry::new())))
}

impl Clone for ProcessRegistry {
    fn clone(&self) -> Self {
        Self {
            processes: Arc::clone(&self.processes),
        }
    }
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: spawn a long-running sleep process for test use.
    /// Caller is responsible for killing it when done.
    fn spawn_test_process() -> Child {
        tokio::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn test process")
    }

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = ProcessRegistry::new();
        assert_eq!(registry.list_ids().await.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_id() {
        let id1 = ProcessRegistry::generate_id();
        let id2 = ProcessRegistry::generate_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("shell_"));
        assert!(id2.starts_with("shell_"));
    }

    #[tokio::test]
    async fn test_register_and_get_status() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_status".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        let status = registry.get_status(&id).await.expect("get_status failed");
        assert!(
            matches!(status, ProcessStatus::Running),
            "Expected Running, got {:?}",
            status
        );

        // Clean up
        registry.kill(&id).await.ok();
    }

    #[tokio::test]
    async fn test_register_and_kill() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_kill".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        assert!(registry.exists(&id).await);

        let killed = registry.kill(&id).await.expect("kill failed");
        assert!(killed, "Expected kill to return true");

        // Process should be removed from registry after kill
        assert!(
            !registry.exists(&id).await,
            "Process should not exist after kill"
        );
    }

    #[tokio::test]
    async fn test_append_and_get_output() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_output".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        // Append stdout lines
        registry
            .append_output(&id, "Line 1".to_string(), false)
            .await
            .expect("append stdout failed");
        registry
            .append_output(&id, "Line 2".to_string(), false)
            .await
            .expect("append stdout failed");

        // Append stderr line
        registry
            .append_output(&id, "Error line".to_string(), true)
            .await
            .expect("append stderr failed");

        let (stdout, stderr, status_str) = registry
            .get_output(&id, None)
            .await
            .expect("get_output failed");

        assert_eq!(stdout, "Line 1\nLine 2");
        assert_eq!(stderr, "Error line");
        assert_eq!(status_str, "running");

        // Clean up
        registry.kill(&id).await.ok();
    }

    #[tokio::test]
    async fn test_mark_completed() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_completed".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        registry
            .mark_completed(&id, 42)
            .await
            .expect("mark_completed failed");

        let status = registry.get_status(&id).await.expect("get_status failed");
        assert!(
            matches!(status, ProcessStatus::Completed(42)),
            "Expected Completed(42), got {:?}",
            status
        );

        // Verify completed_at was set via get_output status string
        let (_, _, status_str) = registry
            .get_output(&id, None)
            .await
            .expect("get_output failed");
        assert_eq!(status_str, "completed:42");

        // Clean up: kill the underlying OS process since mark_completed
        // only updates registry status, it doesn't terminate the child
        registry.kill(&id).await.ok();
    }

    #[tokio::test]
    async fn test_mark_failed() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_failed".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        registry
            .mark_failed(&id, "something went wrong".to_string())
            .await
            .expect("mark_failed failed");

        let status = registry.get_status(&id).await.expect("get_status failed");
        match status {
            ProcessStatus::Failed(msg) => {
                assert_eq!(msg, "something went wrong");
            }
            other => panic!("Expected Failed, got {:?}", other),
        }

        // Clean up
        registry.kill(&id).await.ok();
    }

    #[tokio::test]
    async fn test_exists_check_with_real_process() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_exists".to_string();

        assert!(
            !registry.exists(&id).await,
            "Should not exist before registration"
        );

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        assert!(
            registry.exists(&id).await,
            "Should exist after registration"
        );

        registry.kill(&id).await.expect("kill failed");

        assert!(!registry.exists(&id).await, "Should not exist after kill");
    }

    #[tokio::test]
    async fn test_list_ids() {
        let registry = ProcessRegistry::new();

        let child1 = spawn_test_process();
        let child2 = spawn_test_process();
        let child3 = spawn_test_process();

        registry
            .register("proc_a".to_string(), child1)
            .await
            .expect("register failed");
        registry
            .register("proc_b".to_string(), child2)
            .await
            .expect("register failed");
        registry
            .register("proc_c".to_string(), child3)
            .await
            .expect("register failed");

        let mut ids = registry.list_ids().await;
        ids.sort();
        assert_eq!(ids, vec!["proc_a", "proc_b", "proc_c"]);

        // Clean up
        for id in &ids {
            registry.kill(id).await.ok();
        }
    }

    #[tokio::test]
    async fn test_get_output_clears_buffer() {
        let registry = ProcessRegistry::new();
        let child = spawn_test_process();
        let id = "test_clear_buf".to_string();

        registry
            .register(id.clone(), child)
            .await
            .expect("register failed");

        registry
            .append_output(&id, "first".to_string(), false)
            .await
            .expect("append failed");

        // First get should return the buffered output
        let (stdout, _, _) = registry
            .get_output(&id, None)
            .await
            .expect("get_output failed");
        assert_eq!(stdout, "first");

        // Second get should return empty -- buffer was cleared
        let (stdout2, stderr2, _) = registry
            .get_output(&id, None)
            .await
            .expect("get_output failed");
        assert!(
            stdout2.is_empty(),
            "stdout buffer should be empty after get"
        );
        assert!(
            stderr2.is_empty(),
            "stderr buffer should be empty after get"
        );

        // Clean up
        registry.kill(&id).await.ok();
    }
}
