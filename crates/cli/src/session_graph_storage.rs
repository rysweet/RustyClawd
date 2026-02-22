//! Session graph persistence (load / save / default path)
//!
//! Handles all file I/O for the session graph: locating the storage file,
//! atomic writes via temp-file-then-rename, and loading from JSON.

use crate::session_graph::SessionGraph;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Default filename for session graph storage
const GRAPH_FILENAME: &str = "session_graph.json";

/// Get default storage path for session graph
pub(crate) fn default_storage_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to determine config directory")?
        .join("claude");

    // Ensure config directory exists
    fs::create_dir_all(&config_dir).context("Failed to create config directory")?;

    // Set restrictive permissions (0700) on config directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(&config_dir, perms)
            .with_context(|| format!("Failed to set permissions on {}", config_dir.display()))?;
    }

    Ok(config_dir.join(GRAPH_FILENAME))
}

/// Load graph from file
pub(crate) fn load(path: &Path) -> Result<SessionGraph> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("Failed to read session graph from {}", path.display()))?;

    let mut graph: SessionGraph = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse session graph from {}", path.display()))?;

    graph.set_storage_path(path.to_path_buf());
    Ok(graph)
}

/// Save graph to file (atomic write via temp file + rename)
pub(crate) fn save(graph: &SessionGraph) -> Result<()> {
    let storage_path = graph.storage_path();
    let contents =
        serde_json::to_string_pretty(graph).context("Failed to serialize session graph")?;

    let tmp_path = storage_path.with_extension("json.tmp");
    fs::write(&tmp_path, &contents).with_context(|| {
        format!(
            "Failed to write temp session graph to {}",
            tmp_path.display()
        )
    })?;

    // Set restrictive permissions (0600) on temp file before rename
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&tmp_path, perms)
            .with_context(|| format!("Failed to set permissions on {}", tmp_path.display()))?;
    }

    fs::rename(&tmp_path, storage_path)
        .with_context(|| format!("Failed to rename temp file to {}", storage_path.display()))?;

    Ok(())
}

/// Default graph filename (for fallback paths)
pub(crate) fn graph_filename() -> &'static str {
    GRAPH_FILENAME
}
