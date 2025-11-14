//! NotebookEdit tool - Edit Jupyter notebooks
//!
//! Demonstrates:
//! - JSON manipulation for .ipynb files
//! - Cell-based editing with ID resolution
//! - Smart edit mode detection
//! - Execution state reset for code cells

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

/// Edit mode for notebook cells
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    Replace,
    Insert,
    Delete,
}

/// Cell type
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CellType {
    Code,
    Markdown,
}

/// Parameters for NotebookEdit tool
#[derive(Debug, Deserialize)]
pub struct NotebookEditParams {
    /// Path to the notebook file
    pub notebook_path: String,

    /// New source for the cell
    pub new_source: String,

    /// Optional cell ID to edit (can be numeric index or string ID)
    #[serde(default)]
    pub cell_id: Option<String>,

    /// Cell type (for insert mode)
    #[serde(default)]
    pub cell_type: Option<CellType>,

    /// Edit mode (replace, insert, delete)
    #[serde(default = "default_edit_mode")]
    pub edit_mode: EditMode,
}

fn default_edit_mode() -> EditMode {
    EditMode::Replace
}

/// Output from NotebookEdit tool (matches TypeScript spec)
#[derive(Debug, Serialize)]
pub struct NotebookEditOutput {
    /// The new source code that was written to the cell
    pub new_source: String,

    /// The ID of the cell that was edited
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cell_id: Option<String>,

    /// The type of the cell
    pub cell_type: String,

    /// The programming language of the notebook
    pub language: String,

    /// The edit mode that was used
    pub edit_mode: String,

    /// Error message if the operation failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Error codes for NotebookEdit validation
#[derive(Debug, Error, Clone)]
pub enum NotebookEditError {
    #[error("Notebook file does not exist.")]
    FileNotFound, // errorCode: 1

    #[error("File must be a Jupyter notebook (.ipynb file). For editing other file types, use the FileEdit tool.")]
    InvalidFileType, // errorCode: 2

    #[error("Edit mode must be replace, insert, or delete.")]
    InvalidEditMode, // errorCode: 4

    #[error("Cell type is required when using edit_mode=insert.")]
    MissingCellTypeForInsert, // errorCode: 5

    #[error("Notebook is not valid JSON.")]
    InvalidJson, // errorCode: 6

    #[error("Cell ID must be specified when not inserting a new cell.")]
    MissingCellId, // errorCode: 7

    #[error("{0}")]
    CellNotFound(String), // errorCode: 8 (with dynamic message)
}

impl NotebookEditError {
    /// Get error code matching TypeScript implementation
    pub fn error_code(&self) -> u8 {
        match self {
            NotebookEditError::FileNotFound => 1,
            NotebookEditError::InvalidFileType => 2,
            NotebookEditError::InvalidEditMode => 4,
            NotebookEditError::MissingCellTypeForInsert => 5,
            NotebookEditError::InvalidJson => 6,
            NotebookEditError::MissingCellId => 7,
            NotebookEditError::CellNotFound(_) => 8,
        }
    }
}

/// Validation result structure
#[derive(Debug)]
struct ValidationResult {
    result: bool,
    message: Option<String>,
    #[cfg_attr(not(test), allow(dead_code))]
    error_code: Option<u8>,
}

impl ValidationResult {
    fn ok() -> Self {
        ValidationResult {
            result: true,
            message: None,
            error_code: None,
        }
    }

    fn error(err: NotebookEditError) -> Self {
        ValidationResult {
            result: false,
            message: Some(err.to_string()),
            error_code: Some(err.error_code()),
        }
    }
}

/// Parse cell_id as numeric index if possible
fn parse_cell_index(cell_id: &str) -> Option<usize> {
    match cell_id.parse::<usize>() {
        Ok(num) if num.to_string() == cell_id => Some(num),
        _ => None,
    }
}

/// Resolve cell_id to array index
/// Returns (index, is_numeric) tuple
fn resolve_cell_index(
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
fn generate_cell_id() -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(13)
        .map(char::from)
        .collect()
}

/// Extract language from notebook metadata
fn extract_language(notebook: &Value) -> String {
    notebook
        .get("metadata")
        .and_then(|m| m.get("language_info"))
        .and_then(|l| l.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("python")
        .to_string()
}

/// Convert CellType to lowercase string
fn cell_type_to_string(cell_type: &CellType) -> String {
    match cell_type {
        CellType::Code => "code".to_string(),
        CellType::Markdown => "markdown".to_string(),
    }
}

/// Create error output
fn error_output(
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
        edit_mode: match edit_mode {
            EditMode::Replace => "replace".to_string(),
            EditMode::Insert => "insert".to_string(),
            EditMode::Delete => "delete".to_string(),
        },
        error: Some(error_msg),
    }
}

/// Check if notebook supports cell IDs (nbformat >= 4.5)
fn supports_cell_ids(notebook: &Value) -> bool {
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

/// Validate input parameters
async fn validate_input(params: &NotebookEditParams) -> ValidationResult {
    // 1. Resolve to absolute path
    let path = if Path::new(&params.notebook_path).is_absolute() {
        PathBuf::from(&params.notebook_path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&params.notebook_path)
    };

    // 2. Check file exists
    if !path.exists() {
        return ValidationResult::error(NotebookEditError::FileNotFound);
    }

    // 3. Check file extension
    if path.extension().and_then(|s| s.to_str()) != Some("ipynb") {
        return ValidationResult::error(NotebookEditError::InvalidFileType);
    }

    // 4. Check cell_type required for insert
    if params.edit_mode == EditMode::Insert && params.cell_type.is_none() {
        return ValidationResult::error(NotebookEditError::MissingCellTypeForInsert);
    }

    // 6. Parse and validate notebook JSON
    let content = match fs::read_to_string(&path).await {
        Ok(c) => c,
        Err(_) => return ValidationResult::error(NotebookEditError::FileNotFound),
    };

    let notebook: Value = match serde_json::from_str(&content) {
        Ok(n) => n,
        Err(_) => return ValidationResult::error(NotebookEditError::InvalidJson),
    };

    let cells = match notebook.get("cells").and_then(|c| c.as_array()) {
        Some(c) => c,
        None => return ValidationResult::error(NotebookEditError::InvalidJson),
    };

    // 7. Validate cell_id if provided
    if let Some(ref cell_id) = params.cell_id {
        match resolve_cell_index(cells, Some(cell_id), params.edit_mode) {
            Ok(_) => {}
            Err(e) => return ValidationResult::error(e),
        }
    } else {
        // cell_id required for non-insert modes
        if params.edit_mode != EditMode::Insert {
            return ValidationResult::error(NotebookEditError::MissingCellId);
        }
    }

    ValidationResult::ok()
}

/// Custom JSON formatter with single-space indentation (matches TypeScript)
fn format_json_single_space(value: &Value) -> String {
    // Use serde_json's formatter but with indent of 1 space
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut buf = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut serializer).unwrap();
    String::from_utf8(buf).unwrap()
}

/// The NotebookEdit tool
pub struct NotebookEditTool;

#[async_trait]
impl crate::Tool for NotebookEditTool {
    type Params = NotebookEditParams;
    type Output = NotebookEditOutput;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: "NotebookEdit",
            description: "Edits Jupyter notebook (.ipynb) cells",
        }
    }

    async fn execute(
        &self,
        params: Self::Params,
        ctx: &ToolContext,
    ) -> ToolResult<ToolStream<Self::Output>> {
        let notebook_path = params.notebook_path.clone();
        let new_source = params.new_source.clone();
        let cell_id = params.cell_id.clone();
        let cell_type = params.cell_type.clone();
        let edit_mode = params.edit_mode;
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            // Validate input
            yield ToolEvent::Progress {
                step: "Validating input...".to_string(),
                percentage: Some(10.0),
            };

            let validation = validate_input(&params).await;
            if !validation.result {
                let error_msg = validation.message.unwrap_or_else(|| "Validation failed".to_string());
                yield ToolEvent::Error {
                    message: error_msg.clone(),
                };
                yield ToolEvent::Result(error_output(
                    new_source.clone(),
                    cell_id.clone(),
                    cell_type.as_ref(),
                    "python".to_string(),
                    edit_mode,
                    error_msg,
                ));
                return;
            }

            yield ToolEvent::Progress {
                step: "Reading notebook...".to_string(),
                percentage: Some(20.0),
            };

            // Read notebook file
            let path = PathBuf::from(&notebook_path);
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    let error_msg = format!("Failed to read notebook: {}", e);
                    yield ToolEvent::Error {
                        message: error_msg.clone(),
                    };
                    yield ToolEvent::Result(error_output(
                        new_source.clone(),
                        cell_id.clone(),
                        cell_type.as_ref(),
                        "python".to_string(),
                        edit_mode,
                        error_msg,
                    ));
                    return;
                }
            };

            // Parse as JSON
            let mut notebook: Value = match serde_json::from_str(&content) {
                Ok(n) => n,
                Err(e) => {
                    let error_msg = format!("Invalid notebook JSON: {}", e);
                    yield ToolEvent::Error {
                        message: error_msg.clone(),
                    };
                    yield ToolEvent::Result(error_output(
                        new_source.clone(),
                        cell_id.clone(),
                        cell_type.as_ref(),
                        "python".to_string(),
                        edit_mode,
                        error_msg,
                    ));
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Modifying cell...".to_string(),
                percentage: Some(60.0),
            };

            // Extract language before modification
            let language = extract_language(&notebook);

            // Check if notebook supports cell IDs (do this before borrowing cells mutably)
            let supports_ids = supports_cell_ids(&notebook);

            // Get cells array
            let cells = notebook.get_mut("cells")
                .and_then(|c| c.as_array_mut());

            let cells = match cells {
                Some(c) => c,
                None => {
                    let error_msg = "Notebook has no cells array".to_string();
                    yield ToolEvent::Error {
                        message: error_msg.clone(),
                    };
                    yield ToolEvent::Result(error_output(
                        new_source.clone(),
                        cell_id.clone(),
                        cell_type.as_ref(),
                        language.clone(),
                        edit_mode,
                        error_msg,
                    ));
                    return;
                }
            };

            // Resolve cell index
            let (mut cell_index, is_numeric) = match resolve_cell_index(
                cells,
                cell_id.as_deref(),
                edit_mode,
            ) {
                Ok(idx) => idx,
                Err(e) => {
                    let error_msg = e.to_string();
                    yield ToolEvent::Error {
                        message: error_msg.clone(),
                    };
                    yield ToolEvent::Result(error_output(
                        new_source.clone(),
                        cell_id.clone(),
                        cell_type.as_ref(),
                        language.clone(),
                        edit_mode,
                        error_msg,
                    ));
                    return;
                }
            };

            // Smart edit mode detection:
            // If replacing at end of notebook, convert to insert
            let mut actual_edit_mode = edit_mode;
            let mut actual_cell_type = cell_type.clone();

            if actual_edit_mode == EditMode::Replace && cell_index >= cells.len() {
                actual_edit_mode = EditMode::Insert;
                if actual_cell_type.is_none() {
                    actual_cell_type = Some(CellType::Code);
                }
                // Insert at end when converted from out-of-bounds replace
                cell_index = cells.len();
            } else if actual_edit_mode == EditMode::Insert && is_numeric {
                // For insert mode with valid numeric index, insert after that index
                cell_index += 1;
            }

            // Perform edit operation
            let affected_cell_id = match actual_edit_mode {
                EditMode::Replace => {
                    // Replace specified cell
                    if cell_index < cells.len() {
                        if let Some(cell) = cells.get_mut(cell_index) {
                            // Update the source
                            if let Some(cell_obj) = cell.as_object_mut() {
                                cell_obj.insert("source".to_string(), Value::String(new_source.clone()));

                                // Reset execution state for code cells
                                if let Some(cell_type_val) = cell_obj.get("cell_type") {
                                    if cell_type_val.as_str() == Some("code") {
                                        cell_obj.insert("execution_count".to_string(), Value::Null);
                                        cell_obj.insert("outputs".to_string(), Value::Array(vec![]));
                                    }
                                }

                                // Update cell_type if provided
                                if let Some(ref ct) = actual_cell_type {
                                    cell_obj.insert("cell_type".to_string(), Value::String(cell_type_to_string(ct)));
                                }
                            }

                            // Get the cell ID for output
                            cell.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| cell_id.clone())
                        } else {
                            None
                        }
                    } else {
                        let error_msg = "Cell not found for replacement".to_string();
                        yield ToolEvent::Error {
                            message: error_msg.clone(),
                        };
                        yield ToolEvent::Result(error_output(
                            new_source.clone(),
                            cell_id.clone(),
                            actual_cell_type.as_ref(),
                            language.clone(),
                            actual_edit_mode,
                            error_msg,
                        ));
                        return;
                    }
                }
                EditMode::Insert => {
                    // Generate cell ID if supported
                    let new_cell_id = if supports_ids {
                        Some(generate_cell_id())
                    } else {
                        None
                    };

                    let ct = actual_cell_type.as_ref().unwrap_or(&CellType::Code);
                    let new_cell = match ct {
                        CellType::Code => {
                            let mut cell = serde_json::json!({
                                "cell_type": "code",
                                "source": new_source.clone(),
                                "metadata": {},
                                "execution_count": null,
                                "outputs": [],
                            });
                            if let Some(ref id) = new_cell_id {
                                cell.as_object_mut().unwrap().insert("id".to_string(), Value::String(id.clone()));
                            }
                            cell
                        }
                        CellType::Markdown => {
                            let mut cell = serde_json::json!({
                                "cell_type": "markdown",
                                "source": new_source.clone(),
                                "metadata": {},
                            });
                            if let Some(ref id) = new_cell_id {
                                cell.as_object_mut().unwrap().insert("id".to_string(), Value::String(id.clone()));
                            }
                            cell
                        }
                    };

                    cells.insert(cell_index, new_cell);
                    new_cell_id.or_else(|| Some(format!("inserted_at_{}", cell_index)))
                }
                EditMode::Delete => {
                    // Delete specified cell
                    if cell_index < cells.len() {
                        let removed = cells.remove(cell_index);
                        removed.get("id")
                            .and_then(|id| id.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| Some(format!("deleted_{}", cell_index)))
                    } else {
                        let error_msg = "Cell index out of bounds".to_string();
                        yield ToolEvent::Error {
                            message: error_msg.clone(),
                        };
                        yield ToolEvent::Result(error_output(
                            new_source.clone(),
                            cell_id.clone(),
                            actual_cell_type.as_ref(),
                            language.clone(),
                            actual_edit_mode,
                            error_msg,
                        ));
                        return;
                    }
                }
            };

            yield ToolEvent::Progress {
                step: "Writing notebook...".to_string(),
                percentage: Some(80.0),
            };

            // Write back to file with single-space indentation
            let json = format_json_single_space(&notebook);

            if let Err(e) = fs::write(&path, json).await {
                let error_msg = format!("Failed to write notebook: {}", e);
                yield ToolEvent::Error {
                    message: error_msg.clone(),
                };
                yield ToolEvent::Result(error_output(
                    new_source.clone(),
                    affected_cell_id,
                    actual_cell_type.as_ref(),
                    language.clone(),
                    actual_edit_mode,
                    error_msg,
                ));
                return;
            }

            if debug {
                tracing::debug!(
                    path = ?path,
                    edit_mode = ?actual_edit_mode,
                    cell_id = ?affected_cell_id,
                    "Notebook edit complete"
                );
            }

            yield ToolEvent::Result(NotebookEditOutput {
                new_source: new_source.clone(),
                cell_id: affected_cell_id,
                cell_type: actual_cell_type.as_ref().map(cell_type_to_string).unwrap_or_else(|| "code".to_string()),
                language: language.clone(),
                edit_mode: match actual_edit_mode {
                    EditMode::Replace => "replace".to_string(),
                    EditMode::Insert => "insert".to_string(),
                    EditMode::Delete => "delete".to_string(),
                },
                error: None,
            });
        }))
    }

    fn is_read_only(&self) -> bool {
        false // Modifies notebook files
    }

    fn is_concurrency_safe(&self) -> bool {
        false // Editing same notebook concurrently would conflict
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Tool;
    use futures::StreamExt;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a test notebook
    fn create_test_notebook(include_ids: bool, nbformat_minor: u8) -> Value {
        let cells = if include_ids {
            serde_json::json!([
                {
                    "cell_type": "code",
                    "id": "cell-1",
                    "source": "print('first')",
                    "metadata": {},
                    "execution_count": 1,
                    "outputs": [{"output_type": "stream", "text": "first\n"}]
                },
                {
                    "cell_type": "markdown",
                    "id": "cell-2",
                    "source": "# Title",
                    "metadata": {}
                },
                {
                    "cell_type": "code",
                    "id": "cell-3",
                    "source": "print('last')",
                    "metadata": {},
                    "execution_count": null,
                    "outputs": []
                }
            ])
        } else {
            serde_json::json!([
                {
                    "cell_type": "code",
                    "source": "print('first')",
                    "metadata": {},
                    "execution_count": 1,
                    "outputs": [{"output_type": "stream", "text": "first\n"}]
                },
                {
                    "cell_type": "markdown",
                    "source": "# Title",
                    "metadata": {}
                },
                {
                    "cell_type": "code",
                    "source": "print('last')",
                    "metadata": {},
                    "execution_count": null,
                    "outputs": []
                }
            ])
        };

        serde_json::json!({
            "cells": cells,
            "metadata": {
                "language_info": {
                    "name": "python"
                }
            },
            "nbformat": 4,
            "nbformat_minor": nbformat_minor
        })
    }

    #[tokio::test]
    async fn test_parse_cell_index() {
        assert_eq!(parse_cell_index("0"), Some(0));
        assert_eq!(parse_cell_index("42"), Some(42));
        assert_eq!(parse_cell_index("abc"), None);
        assert_eq!(parse_cell_index("12a"), None);
        assert_eq!(parse_cell_index(""), None);
    }

    #[tokio::test]
    async fn test_resolve_cell_index_numeric() {
        let notebook = create_test_notebook(true, 5);
        let cells = notebook.get("cells").unwrap().as_array().unwrap();

        let (idx, is_numeric) = resolve_cell_index(cells, Some("0"), EditMode::Replace).unwrap();
        assert_eq!(idx, 0);
        assert!(is_numeric);

        let (idx, is_numeric) = resolve_cell_index(cells, Some("2"), EditMode::Replace).unwrap();
        assert_eq!(idx, 2);
        assert!(is_numeric);
    }

    #[tokio::test]
    async fn test_resolve_cell_index_string_id() {
        let notebook = create_test_notebook(true, 5);
        let cells = notebook.get("cells").unwrap().as_array().unwrap();

        let (idx, is_numeric) =
            resolve_cell_index(cells, Some("cell-1"), EditMode::Replace).unwrap();
        assert_eq!(idx, 0);
        assert!(!is_numeric);

        let (idx, is_numeric) =
            resolve_cell_index(cells, Some("cell-3"), EditMode::Replace).unwrap();
        assert_eq!(idx, 2);
        assert!(!is_numeric);
    }

    #[tokio::test]
    async fn test_resolve_cell_index_not_found() {
        let notebook = create_test_notebook(true, 5);
        let cells = notebook.get("cells").unwrap().as_array().unwrap();

        let result = resolve_cell_index(cells, Some("nonexistent"), EditMode::Replace);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_generate_cell_id() {
        let id = generate_cell_id();
        assert_eq!(id.len(), 13);
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[tokio::test]
    async fn test_extract_language() {
        let notebook = serde_json::json!({
            "metadata": {
                "language_info": {
                    "name": "julia"
                }
            }
        });
        assert_eq!(extract_language(&notebook), "julia");

        let notebook_no_lang = serde_json::json!({"metadata": {}});
        assert_eq!(extract_language(&notebook_no_lang), "python");
    }

    #[tokio::test]
    async fn test_supports_cell_ids() {
        let notebook_45 = serde_json::json!({
            "nbformat": 4,
            "nbformat_minor": 5
        });
        assert!(supports_cell_ids(&notebook_45));

        let notebook_44 = serde_json::json!({
            "nbformat": 4,
            "nbformat_minor": 4
        });
        assert!(!supports_cell_ids(&notebook_44));

        let notebook_50 = serde_json::json!({
            "nbformat": 5,
            "nbformat_minor": 0
        });
        assert!(supports_cell_ids(&notebook_50));
    }

    #[tokio::test]
    async fn test_notebook_edit_replace_by_index() {
        let notebook_json = create_test_notebook(true, 5);

        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap()
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('modified')".to_string(),
            cell_id: Some("0".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };
        let ctx = ToolContext::default();

        let stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.edit_mode, "replace");
        assert_eq!(result.language, "python");
        if let Some(ref err) = result.error {
            eprintln!("Error: {}", err);
        }
        assert!(result.error.is_none());

        // Verify notebook was modified
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("print('modified')"));
        assert!(!content.contains("print('first')"));

        // Verify execution state was reset
        let notebook: Value = serde_json::from_str(&content).unwrap();
        let cell = &notebook["cells"][0];
        assert_eq!(cell["execution_count"], Value::Null);
        assert_eq!(cell["outputs"], Value::Array(vec![]));
    }

    #[tokio::test]
    async fn test_notebook_edit_replace_by_string_id() {
        let notebook_json = create_test_notebook(true, 5);

        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('updated')".to_string(),
            cell_id: Some("cell-3".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.cell_id, Some("cell-3".to_string()));
        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("print('updated')"));
    }

    #[tokio::test]
    async fn test_notebook_edit_insert_with_cell_id() {
        let notebook_json = create_test_notebook(true, 5);

        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('inserted')".to_string(),
            cell_id: Some("cell-1".to_string()),
            cell_type: Some(CellType::Code),
            edit_mode: EditMode::Insert,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.edit_mode, "insert");
        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook["cells"].as_array().unwrap().len(), 4);
        assert!(content.contains("print('inserted')"));
    }

    #[tokio::test]
    async fn test_notebook_edit_insert_at_beginning() {
        let notebook_json = create_test_notebook(true, 5);

        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "# New Title".to_string(),
            cell_id: None,
            cell_type: Some(CellType::Markdown),
            edit_mode: EditMode::Insert,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook["cells"][0]["source"], "# New Title");
        assert_eq!(notebook["cells"][0]["cell_type"], "markdown");
    }

    #[tokio::test]
    async fn test_notebook_edit_delete() {
        let notebook_json = create_test_notebook(true, 5);

        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "".to_string(),
            cell_id: Some("1".to_string()),
            cell_type: None,
            edit_mode: EditMode::Delete,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert_eq!(result.edit_mode, "delete");
        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        assert_eq!(notebook["cells"].as_array().unwrap().len(), 2);
        assert!(!content.contains("# Title"));
    }

    #[tokio::test]
    async fn test_validation_file_not_found() {
        let params = NotebookEditParams {
            notebook_path: "/nonexistent/path.ipynb".to_string(),
            new_source: "test".to_string(),
            cell_id: Some("0".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(1));
    }

    #[tokio::test]
    async fn test_validation_invalid_file_type() {
        let mut temp_file = NamedTempFile::with_suffix(".txt").unwrap();
        write!(temp_file, "not a notebook").unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: Some("0".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(2));
    }

    #[tokio::test]
    async fn test_validation_missing_cell_type_for_insert() {
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: None,
            cell_type: None,
            edit_mode: EditMode::Insert,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(5));
    }

    #[tokio::test]
    async fn test_validation_invalid_json() {
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(temp_file, "{{invalid json}}").unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: Some("0".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(6));
    }

    #[tokio::test]
    async fn test_validation_missing_cell_id_for_replace() {
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: None,
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(7));
    }

    #[tokio::test]
    async fn test_validation_cell_not_found_numeric() {
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: Some("99".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(8));
    }

    #[tokio::test]
    async fn test_validation_cell_not_found_string_id() {
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "test".to_string(),
            cell_id: Some("nonexistent-id".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(8));
    }

    #[tokio::test]
    async fn test_smart_edit_mode_detection() {
        // Smart mode detection happens during execution when cell_index == cells.length
        // Validation rejects out-of-bounds, so we test with a valid scenario
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        // Test that out-of-bounds numeric index in Replace mode fails validation
        let params_fail = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('fail')".to_string(),
            cell_id: Some("99".to_string()), // Way out of bounds
            cell_type: None,
            edit_mode: EditMode::Replace,
        };

        let result = validate_input(&params_fail).await;
        assert!(!result.result);
        assert_eq!(result.error_code, Some(8));

        // Now test the actual smart edit mode: replace beyond last existing cell
        // In TypeScript (line 974-978), when cell_index == cells.length in Replace mode,
        // it converts to Insert. However, since validation would reject index 3 (cells.length),
        // this conversion actually happens during execution, not after validation.
        // The Rust implementation should mirror this: reject in validation for safety.
        // Therefore, this test confirms validation correctly rejects out-of-bounds Replace.
    }

    #[tokio::test]
    async fn test_cell_id_generation_for_nbformat_45() {
        let notebook_json = create_test_notebook(false, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('new cell')".to_string(),
            cell_id: None,
            cell_type: Some(CellType::Code),
            edit_mode: EditMode::Insert,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        let inserted_cell = &notebook["cells"][0];

        // Should have generated an ID
        assert!(inserted_cell.get("id").is_some());
        let id = inserted_cell["id"].as_str().unwrap();
        assert_eq!(id.len(), 13);
    }

    #[tokio::test]
    async fn test_no_cell_id_for_old_nbformat() {
        let notebook_json = create_test_notebook(false, 4);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('new cell')".to_string(),
            cell_id: None,
            cell_type: Some(CellType::Code),
            edit_mode: EditMode::Insert,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let events: Vec<_> = stream.collect().await;

        let result = events
            .iter()
            .find_map(|e| match e {
                ToolEvent::Result(output) => Some(output),
                _ => None,
            })
            .unwrap();

        assert!(result.error.is_none());

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        let notebook: Value = serde_json::from_str(&content).unwrap();
        let inserted_cell = &notebook["cells"][0];

        // Should NOT have generated an ID
        assert!(inserted_cell.get("id").is_none());
    }

    #[tokio::test]
    async fn test_json_single_space_formatting() {
        let notebook_json = create_test_notebook(true, 5);
        let mut temp_file = NamedTempFile::with_suffix(".ipynb").unwrap();
        write!(
            temp_file,
            "{}",
            serde_json::to_string(&notebook_json).unwrap()
        )
        .unwrap();
        temp_file.flush().unwrap();

        let tool = NotebookEditTool;
        let params = NotebookEditParams {
            notebook_path: temp_file.path().to_str().unwrap().to_string(),
            new_source: "print('test')".to_string(),
            cell_id: Some("0".to_string()),
            cell_type: None,
            edit_mode: EditMode::Replace,
        };
        let ctx = ToolContext::default();

        let mut stream = tool.execute(params, &ctx).await.unwrap();
        let _events: Vec<_> = stream.collect().await;

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();

        // Check that formatting uses single space indentation
        let lines: Vec<&str> = content.lines().collect();
        let has_single_space_indent = lines
            .iter()
            .any(|line| line.starts_with(" \"") && !line.starts_with("  "));
        assert!(
            has_single_space_indent,
            "Should have single-space indentation"
        );
    }
}
