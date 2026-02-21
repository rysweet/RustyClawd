//! Cell operations and helper functions for NotebookEdit
//!
//! Contains cell index resolution, ID generation, JSON formatting,
//! and other utility functions for notebook cell manipulation.

use rand::Rng;
use serde::Serialize;
use serde_json::Value;

use super::validation::NotebookEditError;
use super::{CellType, EditMode, NotebookEditOutput};

/// Parse cell_id as numeric index if possible
pub fn parse_cell_index(cell_id: &str) -> Option<usize> {
    match cell_id.parse::<usize>() {
        Ok(num) if num.to_string() == cell_id => Some(num),
        _ => None,
    }
}

/// Resolve cell_id to array index
/// Returns (index, is_numeric) tuple
pub fn resolve_cell_index(
    cells: &[Value],
    cell_id: Option<&str>,
    edit_mode: EditMode,
) -> Result<(usize, bool), NotebookEditError> {
    match cell_id {
        None => Ok((0, false)),
        Some(id) => {
            if let Some(idx) = parse_cell_index(id) {
                // Numeric index
                // For Replace/Delete, must be in bounds (Insert allows any index)
                if idx >= cells.len() && edit_mode != EditMode::Insert {
                    return Err(NotebookEditError::CellNotFound(format!(
                        "Cell with index {} does not exist in notebook.",
                        idx
                    )));
                }
                Ok((idx, true))
            } else {
                // String ID lookup
                let idx = cells
                    .iter()
                    .position(|cell| {
                        cell.get("id")
                            .and_then(|v| v.as_str())
                            .map(|s| s == id)
                            .unwrap_or(false)
                    })
                    .ok_or_else(|| {
                        NotebookEditError::CellNotFound(format!(
                            "Cell with ID \"{}\" not found in notebook.",
                            id
                        ))
                    })?;
                Ok((idx, false))
            }
        }
    }
}

/// Generate a random cell ID for nbformat >= 4.5
pub fn generate_cell_id() -> String {
    rand::rng()
        .sample_iter(rand::distr::Alphanumeric)
        .take(13)
        .map(char::from)
        .collect()
}

/// Extract language from notebook metadata
pub fn extract_language(notebook: &Value) -> String {
    notebook
        .get("metadata")
        .and_then(|m| m.get("language_info"))
        .and_then(|l| l.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("python")
        .to_string()
}

/// Convert CellType to lowercase string
pub fn cell_type_to_string(cell_type: &CellType) -> String {
    match cell_type {
        CellType::Code => "code".to_string(),
        CellType::Markdown => "markdown".to_string(),
    }
}

/// Convert EditMode to lowercase string
pub fn edit_mode_to_string(edit_mode: EditMode) -> String {
    match edit_mode {
        EditMode::Replace => "replace".to_string(),
        EditMode::Insert => "insert".to_string(),
        EditMode::Delete => "delete".to_string(),
    }
}

/// Create error output
pub fn error_output(
    new_source: String,
    cell_id: Option<String>,
    cell_type: Option<&CellType>,
    language: String,
    edit_mode: EditMode,
    error_msg: String,
) -> NotebookEditOutput {
    NotebookEditOutput {
        new_source,
        cell_id,
        cell_type: cell_type
            .map(cell_type_to_string)
            .unwrap_or_else(|| "code".to_string()),
        language,
        edit_mode: edit_mode_to_string(edit_mode),
        error: Some(error_msg),
    }
}

/// Check if notebook supports cell IDs (nbformat >= 4.5)
pub fn supports_cell_ids(notebook: &Value) -> bool {
    let nbformat = notebook
        .get("nbformat")
        .and_then(|v| v.as_u64())
        .unwrap_or(4);
    let nbformat_minor = notebook
        .get("nbformat_minor")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    nbformat > 4 || (nbformat == 4 && nbformat_minor >= 5)
}

/// Custom JSON formatter with single-space indentation (matches TypeScript)
pub fn format_json_single_space(value: &Value) -> String {
    // Use serde_json's formatter but with indent of 1 space
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut serializer).unwrap();
    String::from_utf8(buf).unwrap()
}
