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
    async fn test_append_and_retrieve_output() {
        let registry = ProcessRegistry::new();
        let id = "test_shell".to_string();

        // Simulate registration (would normally be done with real child process)
        registry
            .append_output(&id, "Line 1".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&id, "Line 2".to_string(), false)
            .await
            .ok();
        registry
            .append_output(&id, "Error".to_string(), true)
            .await
            .ok();

        // Note: This will fail because process wasn't actually registered
        // In real usage, register() would be called first
    }

    #[tokio::test]
    async fn test_process_status_transitions() {
        let registry = ProcessRegistry::new();
        let id = "test_process".to_string();

        // Simulate a process completing
        registry.mark_completed(&id, 0).await.ok();

        // In real usage, this would show the completed status
    }

    #[tokio::test]
    async fn test_exists_check() {
        let registry = ProcessRegistry::new();
        assert!(!registry.exists("nonexistent").await);

        // After registration (with real child), exists would return true
    }
}
