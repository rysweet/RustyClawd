//! CLI prompt mode - Non-interactive question handling via stdin/stderr
//!
//! Handles single-select and multi-select questions when TUI mode is unavailable.

use super::types::Question;

/// Ask a question in non-interactive/CLI mode
pub(crate) fn ask_cli_mode(question: &Question) -> Result<String, String> {
    eprintln!("\n{}", question.question);
    eprintln!("Options:");
    for (i, opt) in question.options.iter().enumerate() {
        eprintln!("  {}. {} - {}", i + 1, opt.label, opt.description);
    }
    eprintln!("  {}. Other (custom input)", question.options.len() + 1);

    if question.multi_select {
        eprintln!("\nEnter selection(s) (comma-separated numbers or text):");
    } else {
        eprintln!("\nEnter selection (number or text):");
    }

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read input: {}", e))?;

    let input = input.trim();
    if input.is_empty() {
        return Err("No input provided".to_string());
    }

    // Try to parse as number(s)
    if question.multi_select {
        let parts: Vec<&str> = input.split(',').map(|s| s.trim()).collect();
        let mut selected = Vec::new();

        for part in parts {
            if let Ok(num) = part.parse::<usize>() {
                if num > 0 && num <= question.options.len() {
                    selected.push(question.options[num - 1].label.clone());
                } else if num == question.options.len() + 1 {
                    // "Other" selected via number
                    eprintln!("Please specify:");
                    let mut other = String::new();
                    std::io::stdin()
                        .read_line(&mut other)
                        .map_err(|e| format!("Failed to read input: {}", e))?;
                    if !other.trim().is_empty() {
                        selected.push(other.trim().to_string());
                    }
                }
            } else {
                // Treat as custom text
                selected.push(part.to_string());
            }
        }

        if selected.is_empty() {
            return Err("No valid selections".to_string());
        }

        Ok(selected.join(", "))
    } else {
        // Single select
        if let Ok(num) = input.parse::<usize>() {
            if num > 0 && num <= question.options.len() {
                Ok(question.options[num - 1].label.clone())
            } else if num == question.options.len() + 1 {
                // "Other" selected
                eprintln!("Please specify:");
                let mut other = String::new();
                std::io::stdin()
                    .read_line(&mut other)
                    .map_err(|e| format!("Failed to read input: {}", e))?;
                let other = other.trim();
                if other.is_empty() {
                    Err("No input provided".to_string())
                } else {
                    Ok(other.to_string())
                }
            } else {
                Err(format!("Invalid option number: {}", num))
            }
        } else {
            // Treat as custom text
            Ok(input.to_string())
        }
    }
}
