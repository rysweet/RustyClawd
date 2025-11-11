//! Command parser - parses slash command input into structured commands

use anyhow::{anyhow, Result};

/// Parsed command structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// Command name (without leading slash)
    pub name: String,
    /// Arguments as a single string
    pub args_str: Option<String>,
    /// Parsed argument list
    pub args: Vec<String>,
}

impl Command {
    /// Create a new command
    pub fn new(name: String, args_str: Option<String>) -> Self {
        let args = args_str
            .as_ref()
            .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
            .unwrap_or_default();

        Self {
            name,
            args_str,
            args,
        }
    }

    /// Get argument at index
    pub fn get_arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(|s| s.as_str())
    }

    /// Get all arguments as single string
    pub fn args_as_string(&self) -> String {
        self.args_str.clone().unwrap_or_default()
    }
}

/// Command parser
pub struct CommandParser;

impl CommandParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self
    }

    /// Parse a slash command from input
    ///
    /// Format: /command-name [args...]
    /// Examples:
    ///   /help
    ///   /review-pr 123
    ///   /analyze 456 high alice
    pub fn parse(&self, input: &str) -> Result<Command> {
        let trimmed = input.trim();

        // Validate starts with /
        if !trimmed.starts_with('/') {
            return Err(anyhow!("Command must start with /"));
        }

        // Remove leading slash
        let content = trimmed.trim_start_matches('/');

        // Check for empty command
        if content.is_empty() {
            return Err(anyhow!("Command name cannot be empty"));
        }

        // Split into command name and arguments
        let parts: Vec<&str> = content.splitn(2, ' ').collect();
        let command_name = parts[0].to_string();

        // Validate command name (alphanumeric, hyphens, underscores)
        if !self.is_valid_command_name(&command_name) {
            return Err(anyhow!(
                "Invalid command name '{}'. Must contain only alphanumeric characters, hyphens, and underscores",
                command_name
            ));
        }

        let args_str = parts.get(1).map(|s| s.trim().to_string());

        Ok(Command::new(command_name, args_str))
    }

    /// Check if command name is valid
    fn is_valid_command_name(&self, name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ':')
    }
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_no_args() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/help").unwrap();

        assert_eq!(cmd.name, "help");
        assert_eq!(cmd.args_str, None);
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_parse_command_single_arg() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/review-pr 123").unwrap();

        assert_eq!(cmd.name, "review-pr");
        assert_eq!(cmd.args_str, Some("123".to_string()));
        assert_eq!(cmd.args.len(), 1);
        assert_eq!(cmd.get_arg(0), Some("123"));
    }

    #[test]
    fn test_parse_command_multiple_args() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/review-pr 456 high alice").unwrap();

        assert_eq!(cmd.name, "review-pr");
        assert_eq!(cmd.args_str, Some("456 high alice".to_string()));
        assert_eq!(cmd.args.len(), 3);
        assert_eq!(cmd.get_arg(0), Some("456"));
        assert_eq!(cmd.get_arg(1), Some("high"));
        assert_eq!(cmd.get_arg(2), Some("alice"));
    }

    #[test]
    fn test_parse_command_with_hyphens() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/review-pr-detailed 123").unwrap();

        assert_eq!(cmd.name, "review-pr-detailed");
    }

    #[test]
    fn test_parse_command_with_underscores() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/my_custom_command arg").unwrap();

        assert_eq!(cmd.name, "my_custom_command");
    }

    #[test]
    fn test_parse_command_with_namespace() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/amplihack:ultrathink args").unwrap();

        assert_eq!(cmd.name, "amplihack:ultrathink");
        assert_eq!(cmd.args_str, Some("args".to_string()));
    }

    #[test]
    fn test_parse_command_missing_slash() {
        let parser = CommandParser::new();
        let result = parser.parse("help");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_empty_name() {
        let parser = CommandParser::new();
        let result = parser.parse("/ arg");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_command_with_extra_whitespace() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/help   ").unwrap();

        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn test_parse_command_args_with_whitespace() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/cmd   arg1   arg2").unwrap();

        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.get_arg(0), Some("arg1"));
        assert_eq!(cmd.get_arg(1), Some("arg2"));
    }

    #[test]
    fn test_command_args_as_string() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/cmd 123 456 789").unwrap();

        assert_eq!(cmd.args_as_string(), "123 456 789");
    }

    #[test]
    fn test_command_get_arg_out_of_bounds() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/cmd arg1").unwrap();

        assert_eq!(cmd.get_arg(10), None);
    }

    #[test]
    fn test_parse_with_numbers_in_name() {
        let parser = CommandParser::new();
        let cmd = parser.parse("/cmd123 arg").unwrap();

        assert_eq!(cmd.name, "cmd123");
    }

    #[test]
    fn test_parse_empty_input() {
        let parser = CommandParser::new();
        let result = parser.parse("");

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_just_slash() {
        let parser = CommandParser::new();
        let result = parser.parse("/");

        assert!(result.is_err());
    }
}
