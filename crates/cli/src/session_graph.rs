//! Session Graph for Cycle Detection
//!
//! Tracks session parent-child relationships and detects cycles in session chains.
//! Prevents infinite loops when resuming sessions that fork from each other.
//!
//! # Philosophy
//!
//! - **Single Responsibility**: Only handles session chain tracking and cycle detection
//! - **Self-contained**: Manages its own persistence
//! - **Standard library**: No external graph libraries
//! - **Regeneratable**: Can be rebuilt from specification
//!
//! # Public API
//!
//! The "studs" (public interface):
//! - `SessionGraph::new()` - Create/load graph
//! - `add_edge()` - Record parent-child relationship
//! - `detect_cycle()` - Check for cycles starting from session
//! - `get_chain()` - Get full ancestry chain for session
//! - `max_depth()` - Calculate maximum depth from session
//!
//! # Example
//!
//! ```rust,ignore
//! use rustyclawd_cli::session_graph::SessionGraph;
//!
//! let mut graph = SessionGraph::new()?;
//!
//! // session-2 forks from session-1
//! graph.add_edge("session-2", "session-1")?;
//!
//! // session-3 forks from session-2
//! graph.add_edge("session-3", "session-2")?;
//!
//! // Get full chain: [session-3, session-2, session-1]
//! let chain = graph.get_chain("session-3");
//!
//! // Check for cycles (none in this case)
//! assert!(graph.detect_cycle("session-3").is_none());
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum allowed depth for session chains
pub const MAX_CHAIN_DEPTH: usize = 100;

/// Default filename for session graph storage
const GRAPH_FILENAME: &str = "session_graph.json";

/// Session graph for tracking parent-child relationships
///
/// Maintains directed graph structure:
/// - child → parent (primary direction)
/// - parent → children (for efficient traversal)
///
/// Persisted to disk as JSON for durability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGraph {
    /// Map from child session ID to parent session ID
    #[serde(default)]
    edges: HashMap<String, String>,

    /// Map from parent session ID to list of child session IDs
    #[serde(default)]
    children: HashMap<String, Vec<String>>,

    /// Last update timestamp (ISO 8601 format)
    #[serde(default)]
    last_updated: Option<String>,

    /// Path where this graph is persisted
    #[serde(skip)]
    storage_path: PathBuf,
}

/// Result of cycle detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cycle {
    /// The cycle path (first and last elements are the same)
    pub path: Vec<String>,
}

impl Cycle {
    /// Format cycle for display
    pub fn format(&self) -> String {
        self.path.join(" → ")
    }

    /// Get the repeated node (the one that creates the cycle)
    pub fn repeated_node(&self) -> Option<&str> {
        self.path.first().map(|s| s.as_str())
    }
}

impl SessionGraph {
    /// Create a new session graph or load existing from default location
    ///
    /// Creates graph file if it doesn't exist. Uses `~/.config/claude/session_graph.json`.
    pub fn new() -> Result<Self> {
        let storage_path = Self::default_storage_path()?;
        Self::from_path(storage_path)
    }

    /// Create session graph from specific path
    pub fn from_path(storage_path: PathBuf) -> Result<Self> {
        if storage_path.exists() {
            Self::load(&storage_path)
        } else {
            Self::create_empty(storage_path)
        }
    }

    /// Get default storage path for session graph
    fn default_storage_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Failed to determine config directory")?
            .join("claude");

        // Ensure config directory exists
        fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

        Ok(config_dir.join(GRAPH_FILENAME))
    }

    /// Create empty graph at specified path
    fn create_empty(storage_path: PathBuf) -> Result<Self> {
        let graph = Self {
            edges: HashMap::new(),
            children: HashMap::new(),
            last_updated: None,
            storage_path,
        };

        graph.save()?;
        Ok(graph)
    }

    /// Load graph from file
    fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .with_context(|| format!("Failed to read session graph from {}", path.display()))?;

        let mut graph: Self = serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse session graph from {}", path.display()))?;

        graph.storage_path = path.to_path_buf();
        Ok(graph)
    }

    /// Save graph to file (atomic write via temp file + rename)
    fn save(&self) -> Result<()> {
        let contents =
            serde_json::to_string_pretty(self).context("Failed to serialize session graph")?;

        let tmp_path = self.storage_path.with_extension("json.tmp");
        fs::write(&tmp_path, &contents).with_context(|| {
            format!(
                "Failed to write temp session graph to {}",
                tmp_path.display()
            )
        })?;

        fs::rename(&tmp_path, &self.storage_path).with_context(|| {
            format!(
                "Failed to rename temp file to {}",
                self.storage_path.display()
            )
        })?;

        Ok(())
    }

    /// Add a parent-child edge to the graph
    ///
    /// Records that `child` was forked from `parent`.
    /// If child already has a parent, the old edge is replaced.
    ///
    /// # Arguments
    ///
    /// * `child` - Child session ID
    /// * `parent` - Parent session ID
    ///
    /// # Errors
    ///
    /// Returns error if adding this edge would create a cycle.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// graph.add_edge("session-2", "session-1")?;
    /// ```
    pub fn add_edge(&mut self, child: &str, parent: &str) -> Result<()> {
        // Temporarily add edge to check for cycles
        let old_parent = self.edges.insert(child.to_string(), parent.to_string());

        // Check for cycles
        if let Some(cycle) = self.detect_cycle(child) {
            // Restore old state
            if let Some(old) = old_parent {
                self.edges.insert(child.to_string(), old);
            } else {
                self.edges.remove(child);
            }

            anyhow::bail!("Adding edge would create cycle: {}", cycle.format());
        }

        // Check maximum depth
        let depth = self.max_depth(child);
        if depth >= MAX_CHAIN_DEPTH {
            // Restore old state
            if let Some(old) = old_parent {
                self.edges.insert(child.to_string(), old);
            } else {
                self.edges.remove(child);
            }

            anyhow::bail!(
                "Session chain too deep (max {} sessions, got {})",
                MAX_CHAIN_DEPTH,
                depth
            );
        }

        // Remove child from old parent's children list (if exists)
        if let Some(old_parent_id) = &old_parent {
            if let Some(siblings) = self.children.get_mut(old_parent_id) {
                siblings.retain(|s| s != child);
                if siblings.is_empty() {
                    self.children.remove(old_parent_id);
                }
            }
        }

        // Add to children map
        self.children
            .entry(parent.to_string())
            .or_default()
            .push(child.to_string());

        // Update timestamp and save
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
        self.save()?;

        Ok(())
    }

    /// Detect cycle starting from a session
    ///
    /// Follows the parent chain from session_id. Since each node has at most
    /// one parent, this is a simple linked-list traversal with a visited set.
    /// Returns the cycle path if found, or None if no cycle.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(cycle) = graph.detect_cycle("session-3") {
    ///     eprintln!("Cycle detected: {}", cycle.format());
    /// }
    /// ```
    pub fn detect_cycle(&self, session_id: &str) -> Option<Cycle> {
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut current = session_id;

        loop {
            if visited.contains(current) {
                // Found a cycle - extract the cycle portion of the path
                let cycle_start = path.iter().position(|s: &String| s == current).unwrap();
                let mut cycle_path = path[cycle_start..].to_vec();
                cycle_path.push(current.to_string()); // Close the cycle
                return Some(Cycle { path: cycle_path });
            }

            visited.insert(current.to_string());
            path.push(current.to_string());

            match self.edges.get(current) {
                Some(parent) => current = parent,
                None => return None, // Reached a root, no cycle
            }
        }
    }

    /// Get full ancestry chain for a session
    ///
    /// Returns list of session IDs from current session to root ancestor.
    /// First element is the session itself, last element is the root.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let chain = graph.get_chain("session-3");
    /// // Returns: ["session-3", "session-2", "session-1"]
    /// ```
    pub fn get_chain(&self, session_id: &str) -> Vec<String> {
        let mut chain = vec![session_id.to_string()];
        let mut current = session_id;

        while let Some(parent) = self.edges.get(current) {
            chain.push(parent.clone());
            current = parent;

            // Safety: Prevent infinite loop if graph is corrupted
            if chain.len() > MAX_CHAIN_DEPTH {
                break;
            }
        }

        chain
    }

    /// Calculate maximum depth from a session to root
    ///
    /// Returns the number of edges in the longest path from session to a root.
    pub fn max_depth(&self, session_id: &str) -> usize {
        self.get_chain(session_id).len() - 1
    }

    /// Get parent of a session (if exists)
    pub fn get_parent(&self, session_id: &str) -> Option<&str> {
        self.edges.get(session_id).map(|s| s.as_str())
    }

    /// Get children of a session
    pub fn get_children(&self, session_id: &str) -> Option<&[String]> {
        self.children.get(session_id).map(|v| v.as_slice())
    }

    /// Get all root sessions (sessions with no parents)
    pub fn get_roots(&self) -> Vec<String> {
        let all_sessions: HashSet<String> = self
            .edges
            .keys()
            .chain(self.children.keys())
            .cloned()
            .collect();

        all_sessions
            .into_iter()
            .filter(|s| !self.edges.contains_key(s))
            .collect()
    }

    /// Remove a session from the graph
    ///
    /// Removes the session and all references to it.
    /// Children of the removed session become roots (their parent edges are removed).
    pub fn remove_session(&mut self, session_id: &str) -> Result<()> {
        // Remove from edges (if it's a child)
        if let Some(parent_id) = self.edges.remove(session_id) {
            // Remove from parent's children list
            if let Some(siblings) = self.children.get_mut(&parent_id) {
                siblings.retain(|s| s != session_id);
                if siblings.is_empty() {
                    self.children.remove(&parent_id);
                }
            }
        }

        // Remove children's edges that point to this session (make them roots)
        if let Some(child_ids) = self.children.remove(session_id) {
            for child_id in &child_ids {
                self.edges.remove(child_id);
            }
        }

        // Update timestamp and save
        self.last_updated = Some(chrono::Utc::now().to_rfc3339());
        self.save()?;

        Ok(())
    }

    /// Get total number of sessions in graph
    pub fn session_count(&self) -> usize {
        let mut sessions: HashSet<&String> = HashSet::new();
        sessions.extend(self.edges.keys());
        sessions.extend(self.edges.values());
        sessions.extend(self.children.keys());
        sessions.len()
    }
}

impl Default for SessionGraph {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to a temp directory if config dir is unavailable
            let tmp_path = std::env::temp_dir().join(GRAPH_FILENAME);
            Self {
                edges: HashMap::new(),
                children: HashMap::new(),
                last_updated: None,
                storage_path: tmp_path,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_graph() -> (SessionGraph, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("test_graph.json");
        let graph = SessionGraph::from_path(graph_path).unwrap();
        (graph, temp_dir)
    }

    #[test]
    fn test_add_edge_basic() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("child", "parent").unwrap();

        assert_eq!(graph.get_parent("child"), Some("parent"));
        assert_eq!(
            graph.get_children("parent"),
            Some(&["child".to_string()][..])
        );
    }

    #[test]
    fn test_detect_cycle_simple() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("session-2", "session-1").unwrap();

        // This would create a cycle: session-1 -> session-2 -> session-1
        let result = graph.add_edge("session-1", "session-2");
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_cycle_complex() {
        let (mut graph, _temp) = create_test_graph();

        // Create chain: A -> B -> C
        graph.add_edge("B", "A").unwrap();
        graph.add_edge("C", "B").unwrap();

        // Try to create cycle: A -> B -> C -> A
        let result = graph.add_edge("A", "C");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_chain() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("session-3", "session-2").unwrap();
        graph.add_edge("session-2", "session-1").unwrap();

        let chain = graph.get_chain("session-3");
        assert_eq!(chain, vec!["session-3", "session-2", "session-1"]);
    }

    #[test]
    fn test_max_depth() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("session-3", "session-2").unwrap();
        graph.add_edge("session-2", "session-1").unwrap();

        assert_eq!(graph.max_depth("session-3"), 2);
        assert_eq!(graph.max_depth("session-2"), 1);
        assert_eq!(graph.max_depth("session-1"), 0);
    }

    #[test]
    fn test_max_depth_limit() {
        let (mut graph, _temp) = create_test_graph();

        // Create chain just under the limit
        for i in 1..MAX_CHAIN_DEPTH {
            graph
                .add_edge(&format!("session-{}", i + 1), &format!("session-{}", i))
                .unwrap();
        }

        // This should fail - would exceed MAX_CHAIN_DEPTH
        let result = graph.add_edge("session-new", &format!("session-{}", MAX_CHAIN_DEPTH));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too deep"));
    }

    #[test]
    fn test_get_roots() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("B", "A").unwrap();
        graph.add_edge("C", "A").unwrap();
        graph.add_edge("E", "D").unwrap();

        let mut roots = graph.get_roots();
        roots.sort();
        assert_eq!(roots, vec!["A", "D"]);
    }

    #[test]
    fn test_remove_session() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("child-1", "parent").unwrap();
        graph.add_edge("child-2", "parent").unwrap();

        graph.remove_session("child-1").unwrap();

        assert!(graph.get_parent("child-1").is_none());
        assert_eq!(
            graph.get_children("parent"),
            Some(&["child-2".to_string()][..])
        );
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("test_graph.json");

        // Create and populate graph
        {
            let mut graph = SessionGraph::from_path(graph_path.clone()).unwrap();
            graph.add_edge("B", "A").unwrap();
            graph.add_edge("C", "B").unwrap();
        }

        // Load and verify
        {
            let graph = SessionGraph::from_path(graph_path).unwrap();
            assert_eq!(graph.get_parent("B"), Some("A"));
            assert_eq!(graph.get_parent("C"), Some("B"));

            let chain = graph.get_chain("C");
            assert_eq!(chain, vec!["C", "B", "A"]);
        }
    }

    #[test]
    fn test_replace_parent() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("child", "parent-1").unwrap();
        assert_eq!(graph.get_parent("child"), Some("parent-1"));

        // Replace parent
        graph.add_edge("child", "parent-2").unwrap();
        assert_eq!(graph.get_parent("child"), Some("parent-2"));

        // parent-1 should no longer have child
        assert!(graph.get_children("parent-1").is_none());

        // parent-2 should have child
        assert_eq!(
            graph.get_children("parent-2"),
            Some(&["child".to_string()][..])
        );
    }

    #[test]
    fn test_multiple_children() {
        let (mut graph, _temp) = create_test_graph();

        graph.add_edge("child-1", "parent").unwrap();
        graph.add_edge("child-2", "parent").unwrap();
        graph.add_edge("child-3", "parent").unwrap();

        let children = graph.get_children("parent").unwrap();
        assert_eq!(children.len(), 3);
        assert!(children.contains(&"child-1".to_string()));
        assert!(children.contains(&"child-2".to_string()));
        assert!(children.contains(&"child-3".to_string()));
    }

    #[test]
    fn test_remove_middle_of_chain() {
        let (mut graph, _temp) = create_test_graph();

        // Create chain: A -> B -> C
        graph.add_edge("B", "A").unwrap();
        graph.add_edge("C", "B").unwrap();

        assert_eq!(graph.get_chain("C"), vec!["C", "B", "A"]);

        // Remove B (middle of chain)
        graph.remove_session("B").unwrap();

        // C should now be a root (no dangling reference to B)
        assert!(graph.get_parent("C").is_none());
        assert_eq!(graph.get_chain("C"), vec!["C"]);

        // B should be completely gone
        assert!(graph.get_parent("B").is_none());
        assert!(graph.get_children("B").is_none());

        // A should still exist as a standalone root
        assert!(graph.get_parent("A").is_none());
    }

    #[test]
    fn test_corrupted_json_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let graph_path = temp_dir.path().join("test_graph.json");

        // Write invalid JSON to the file
        fs::write(&graph_path, "{ not valid json !!!").unwrap();

        // Loading should return an error, not panic
        let result = SessionGraph::from_path(graph_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Failed to parse"),
            "Error should mention parsing failure, got: {}",
            err_msg
        );
    }
}
