//! NotebookEdit tool - Edit Jupyter notebooks
//!
//! Demonstrates:
//! - JSON manipulation for .ipynb files
//! - Cell-based editing with ID resolution
//! - Smart edit mode detection
//! - Execution state reset for code cells

pub mod cell_ops;
pub mod validation;

#[cfg(test)]
mod tests;

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use cell_ops::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;
use validation::validate_input;

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
                edit_mode: edit_mode_to_string(actual_edit_mode),
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
