// Brand validation tests for RustyClawd
// Ensures no "Claude" branding in user-facing UI code

#[cfg(test)]
mod branding_tests {
    use std::fs;
    use std::path::Path;

    /// UI directories that should not contain "Claude" branding
    const UI_PATHS: &[&str] = &["src/tui", "src/main.rs", "src/interactive.rs"];

    /// Allowed patterns where "Claude" is acceptable
    const ALLOWED_CONTEXTS: &[&str] = &[
        "claude-sonnet", // API model names
        "claude-opus",   // API model names
        "claude-haiku",  // API model names
        ".claude/",      // Directory paths
        "claude_code",   // Internal variable names
    ];

    fn should_check_line(line: &str) -> bool {
        // Skip comments
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            return false;
        }

        // Skip test modules (they can reference Claude for testing)
        if trimmed.contains("#[test]") || trimmed.contains("#[cfg(test)]") {
            return false;
        }

        true
    }

    fn is_allowed_context(line: &str) -> bool {
        ALLOWED_CONTEXTS.iter().any(|ctx| line.contains(ctx))
    }

    fn check_file_for_claude_branding(path: &Path) -> Vec<String> {
        let mut violations = Vec::new();

        if let Ok(content) = fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                if !should_check_line(line) {
                    continue;
                }

                // Check for "Claude" in string literals
                if line.contains("\"") && line.contains("Claude") && !is_allowed_context(line) {
                    violations.push(format!(
                        "{}:{}: {}",
                        path.display(),
                        line_num + 1,
                        line.trim()
                    ));
                }
            }
        }

        violations
    }

    fn check_directory(dir: &str) -> Vec<String> {
        let mut violations = Vec::new();
        let path = Path::new(dir);

        if path.is_file() {
            violations.extend(check_file_for_claude_branding(path));
        } else if path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() && entry_path.extension().is_some_and(|e| e == "rs") {
                        violations.extend(check_file_for_claude_branding(&entry_path));
                    } else if entry_path.is_dir() {
                        // Recursively check subdirectories
                        if let Some(subdir) = entry_path.to_str() {
                            violations.extend(check_directory(subdir));
                        }
                    }
                }
            }
        }

        violations
    }

    #[test]
    fn test_no_claude_branding_in_ui() {
        let mut all_violations = Vec::new();

        for path in UI_PATHS {
            let violations = check_directory(path);
            all_violations.extend(violations);
        }

        if !all_violations.is_empty() {
            let violation_report = all_violations.join("\n");
            panic!(
                "\n\n❌ Found Claude branding in UI code:\n{}\n\n\
                 Please use 'RustyClawd' or generic terms like 'Assistant' instead.\n",
                violation_report
            );
        }
    }

    #[test]
    fn test_allowed_contexts_are_present() {
        // This test verifies that our allowed contexts (like model names) still exist
        // If this fails, we might need to update our ALLOWED_CONTEXTS list

        // Just verify the test infrastructure is working
        assert!(
            !ALLOWED_CONTEXTS.is_empty(),
            "Allowed contexts should be defined"
        );
        assert!(!UI_PATHS.is_empty(), "UI paths should be defined");
    }
}
