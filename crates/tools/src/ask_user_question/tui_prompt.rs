//! TUI prompt mode - Interactive terminal prompts using dialoguer
//!
//! Handles single-select and multi-select questions with automatic "Other" option support.

use super::types::Question;
use dialoguer::{theme::ColorfulTheme, Input, MultiSelect, Select};

/// Ask a single-select question in TUI mode
pub(crate) fn ask_single_select_tui(question: &Question, debug: bool) -> Result<String, String> {
    // Add "Other" option automatically
    let mut items: Vec<String> = question
        .options
        .iter()
        .map(|opt| format!("{} - {}", opt.label, opt.description))
        .collect();
    items.push("Other (custom input)".to_string());

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(&question.question)
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| {
            if debug {
                tracing::warn!("User cancelled or error: {}", e);
            }
            format!("Question cancelled or error: {}", e)
        })?;

    // Handle "Other" option
    if selection == items.len() - 1 {
        let other: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Please specify")
            .interact_text()
            .map_err(|e| format!("Failed to read input: {}", e))?;

        if other.trim().is_empty() {
            return Err("No input provided".to_string());
        }
        Ok(other.trim().to_string())
    } else {
        Ok(question.options[selection].label.clone())
    }
}

/// Ask a multi-select question in TUI mode
pub(crate) fn ask_multi_select_tui(question: &Question, debug: bool) -> Result<String, String> {
    // Add "Other" option automatically
    let mut items: Vec<String> = question
        .options
        .iter()
        .map(|opt| format!("{} - {}", opt.label, opt.description))
        .collect();
    items.push("Other (custom input)".to_string());

    let selections = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt(&question.question)
        .items(&items)
        .interact()
        .map_err(|e| {
            if debug {
                tracing::warn!("User cancelled or error: {}", e);
            }
            format!("Question cancelled or error: {}", e)
        })?;

    if selections.is_empty() {
        return Err("No options selected".to_string());
    }

    let mut selected_labels = Vec::new();

    // Check if "Other" was selected
    let other_selected = selections.contains(&(items.len() - 1));

    // Collect regular selections
    for &idx in &selections {
        if idx < question.options.len() {
            selected_labels.push(question.options[idx].label.clone());
        }
    }

    // Handle "Other" option
    if other_selected {
        let other: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Please specify other option(s)")
            .interact_text()
            .map_err(|e| format!("Failed to read input: {}", e))?;

        if !other.trim().is_empty() {
            selected_labels.push(other.trim().to_string());
        }
    }

    Ok(selected_labels.join(", "))
}
