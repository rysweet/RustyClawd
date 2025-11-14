//! NotebookEdit tool - Edit Jupyter notebooks
//!
//! Demonstrates:
//! - JSON manipulation for .ipynb files
//! - Cell-based editing
//! - Preserving notebook metadata

use crate::{ToolContext, ToolEvent, ToolMetadata, ToolResult, ToolStream};
use async_stream::stream;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

/// Edit mode for notebook cells
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum EditMode {
    Replace,
    Insert,
    Delete,
}

/// Cell type
#[derive(Debug, Deserialize, Serialize, Clone)]
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

    /// Optional cell ID to edit
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

/// Output from NotebookEdit tool
#[derive(Debug, Serialize)]
pub struct NotebookEditOutput {
    /// Notebook path that was edited
    pub notebook_path: String,

    /// Edit mode used
    pub edit_mode: String,

    /// Cell ID affected
    pub cell_id: Option<String>,

    /// Success message
    pub message: String,
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
        let path = PathBuf::from(&params.notebook_path);
        let new_source = params.new_source.clone();
        let cell_id = params.cell_id.clone();
        let edit_mode = params.edit_mode;
        let debug = ctx.debug;

        Ok(Box::pin(stream! {
            yield ToolEvent::Progress {
                step: "Reading notebook...".to_string(),
                percentage: Some(20.0),
            };

            // Read notebook file
            let content = match fs::read_to_string(&path).await {
                Ok(c) => c,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to read notebook: {}", e),
                    };
                    return;
                }
            };

            // Parse as JSON
            let mut notebook: Value = match serde_json::from_str(&content) {
                Ok(n) => n,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Invalid notebook JSON: {}", e),
                    };
                    return;
                }
            };

            yield ToolEvent::Progress {
                step: "Modifying cell...".to_string(),
                percentage: Some(60.0),
            };

            // Get cells array
            let cells = notebook.get_mut("cells")
                .and_then(|c| c.as_array_mut());

            let cells = match cells {
                Some(c) => c,
                None => {
                    yield ToolEvent::Error {
                        message: "Notebook has no cells array".to_string(),
                    };
                    return;
                }
            };

            // Find cell index by ID or use first cell
            let cell_index = if let Some(ref target_id) = cell_id {
                // Find cell by ID
                cells.iter().position(|cell| {
                    cell.get("id")
                        .and_then(|id| id.as_str())
                        .map(|id| id == target_id)
                        .unwrap_or(false)
                })
            } else {
                // No ID specified, use first cell
                if !cells.is_empty() { Some(0) } else { None }
            };

            // Perform edit based on mode
            let affected_cell_id = match edit_mode {
                EditMode::Replace => {
                    // Replace specified cell
                    if let Some(idx) = cell_index {
                        if let Some(cell) = cells.get_mut(idx) {
                            // Update the source
                            if let Some(source) = cell.get_mut("source") {
                                *source = Value::String(new_source.clone());
                            } else {
                                cell.as_object_mut().unwrap().insert("source".to_string(), Value::String(new_source.clone()));
                            }

                            // Get the cell ID for output
                            cell.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| Some(idx.to_string()))
                        } else {
                            None
                        }
                    } else {
                        yield ToolEvent::Error {
                            message: "Cell not found for replacement".to_string(),
                        };
                        return;
                    }
                }
                EditMode::Insert => {
                    // Insert new cell after specified cell, or at beginning
                    let insert_idx = cell_index.map(|i| i + 1).unwrap_or(0);

                    let new_cell = serde_json::json!({
                        "cell_type": params.cell_type.unwrap_or(CellType::Code),
                        "source": new_source,
                        "metadata": {},
                        "outputs": [],
                    });

                    cells.insert(insert_idx, new_cell);
                    Some(format!("inserted_at_{}", insert_idx))
                }
                EditMode::Delete => {
                    // Delete specified cell
                    if let Some(idx) = cell_index {
                        if idx < cells.len() {
                            let removed = cells.remove(idx);
                            removed.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s.to_string())
                                .or_else(|| Some(format!("deleted_{}", idx)))
                        } else {
                            yield ToolEvent::Error {
                                message: "Cell index out of bounds".to_string(),
                            };
                            return;
                        }
                    } else {
                        yield ToolEvent::Error {
                            message: "No cell specified for deletion".to_string(),
                        };
                        return;
                    }
                }
            };

            yield ToolEvent::Progress {
                step: "Writing notebook...".to_string(),
                percentage: Some(80.0),
            };

            // Write back to file
            let json = match serde_json::to_string_pretty(&notebook) {
                Ok(j) => j,
                Err(e) => {
                    yield ToolEvent::Error {
                        message: format!("Failed to serialize notebook: {}", e),
                    };
                    return;
                }
            };

            if let Err(e) = fs::write(&path, json).await {
                yield ToolEvent::Error {
                    message: format!("Failed to write notebook: {}", e),
                };
                return;
            }

            let message = format!("Notebook edited successfully ({:?} mode)", edit_mode);

            if debug {
                tracing::debug!(
                    path = ?path,
                    edit_mode = ?edit_mode,
                    cell_id = ?affected_cell_id,
                    "Notebook edit complete"
                );
            }

            yield ToolEvent::Result(NotebookEditOutput {
                notebook_path: params.notebook_path.clone(),
                edit_mode: format!("{:?}", edit_mode),
                cell_id: affected_cell_id,
                message,
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

    #[tokio::test]
    async fn test_notebook_edit_replace() {
        // Create minimal notebook
        let notebook_json = serde_json::json!({
            "cells": [
                {
                    "cell_type": "code",
                    "source": "print('old')",
                    "metadata": {},
                    "outputs": []
                }
            ],
            "metadata": {},
            "nbformat": 4,
            "nbformat_minor": 5
        });

        let mut temp_file = NamedTempFile::new().unwrap();
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
            new_source: "print('new')".to_string(),
            cell_id: None,
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

        assert_eq!(result.edit_mode, "Replace");

        // Verify notebook was modified
        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("print('new')"));
        assert!(!content.contains("print('old')"));
    }
}
