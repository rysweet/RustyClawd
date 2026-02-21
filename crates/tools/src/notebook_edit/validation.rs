//! Validation logic for NotebookEdit tool
//!
//! Contains input validation, error types, and validation result structures.

use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

use super::cell_ops::resolve_cell_index;
use super::{EditMode, NotebookEditParams};

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
pub struct ValidationResult {
    pub result: bool,
    pub message: Option<String>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub error_code: Option<u8>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        ValidationResult {
            result: true,
            message: None,
            error_code: None,
        }
    }

    pub fn error(err: NotebookEditError) -> Self {
        ValidationResult {
            result: false,
            message: Some(err.to_string()),
            error_code: Some(err.error_code()),
        }
    }
}

/// Validate input parameters
pub async fn validate_input(params: &NotebookEditParams) -> ValidationResult {
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
