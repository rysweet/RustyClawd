//! TestSkillEnvironment - Temporary skill directories for testing
//!
//! Creates temporary skill directories with test skill files for E2E testing.
//! Uses RAII pattern for automatic cleanup.
//!
//! Philosophy:
//! - Single responsibility: Manage test skill files
//! - Zero-BS: Real skill files on disk
//! - Self-contained: Cleanup happens automatically
//! - Regeneratable: Can recreate skill environment from spec

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Test skill environment with automatic cleanup
///
/// Creates a temporary directory with skill files for testing.
/// Automatically cleans up on drop (RAII pattern).
///
/// # Example
///
/// ```no_run
/// use rustyclawd_cli::e2e::helpers::TestSkillEnvironment;
///
/// let skill_env = TestSkillEnvironment::new()
///     .with_skill("test-analyzer", "Perform deep analysis")
///     .build()?;
///
/// // Use skill_env.path() in tests
/// // Cleanup happens automatically when skill_env is dropped
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct TestSkillEnvironment {
    temp_dir: Option<PathBuf>,
    skills: Vec<(String, String, Option<String>)>, // (name, content, frontmatter)
}

impl TestSkillEnvironment {
    /// Create new test skill environment builder
    pub fn new() -> Self {
        Self {
            temp_dir: None,
            skills: Vec::new(),
        }
    }

    /// Add a simple skill with just content
    ///
    /// # Example
    ///
    /// ```no_run
    /// let env = TestSkillEnvironment::new()
    ///     .with_skill("analyzer", "Analyze the code")
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_skill(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.skills.push((name.into(), content.into(), None));
        self
    }

    /// Add a skill with frontmatter and content
    ///
    /// # Example
    ///
    /// ```no_run
    /// let env = TestSkillEnvironment::new()
    ///     .with_skill_full(
    ///         "analyzer",
    ///         "---\ntype: skill\n---",
    ///         "Analyze the code"
    ///     )
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_skill_full(
        mut self,
        name: impl Into<String>,
        frontmatter: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        self.skills
            .push((name.into(), content.into(), Some(frontmatter.into())));
        self
    }

    /// Build the skill environment (creates temp directory and files)
    pub fn build(mut self) -> Result<TestSkillEnvGuard> {
        // Create temporary directory
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path().to_path_buf();

        // Create skill files
        for (name, content, frontmatter) in &self.skills {
            let skill_path = temp_path.join(format!("{}.md", name));

            let full_content = if let Some(fm) = frontmatter {
                format!("{}\n\n{}", fm, content)
            } else {
                content.clone()
            };

            fs::write(&skill_path, full_content)?;
        }

        self.temp_dir = Some(temp_path.clone());

        Ok(TestSkillEnvGuard {
            _temp_dir: temp_dir,
            path: temp_path,
        })
    }
}

impl Default for TestSkillEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard for test skill environment with automatic cleanup
///
/// When dropped, automatically removes the temporary directory and all files.
/// This is the RAII pattern in action.
pub struct TestSkillEnvGuard {
    _temp_dir: tempfile::TempDir, // Cleanup happens when this is dropped
    path: PathBuf,
}

impl TestSkillEnvGuard {
    /// Get path to skill directory
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// Cleanup happens automatically via Drop trait on tempfile::TempDir

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_skill_environment() {
        let env = TestSkillEnvironment::new()
            .with_skill("test-skill", "Test content")
            .build()
            .unwrap();

        // Verify directory exists
        assert!(env.path().exists());
        assert!(env.path().is_dir());

        // Verify skill file exists
        let skill_file = env.path().join("test-skill.md");
        assert!(skill_file.exists());

        // Verify content
        let content = fs::read_to_string(skill_file).unwrap();
        assert_eq!(content, "Test content");
    }

    #[test]
    fn test_multiple_skills() {
        let env = TestSkillEnvironment::new()
            .with_skill("skill1", "Content 1")
            .with_skill("skill2", "Content 2")
            .with_skill("skill3", "Content 3")
            .build()
            .unwrap();

        // Verify all skill files exist
        assert!(env.path().join("skill1.md").exists());
        assert!(env.path().join("skill2.md").exists());
        assert!(env.path().join("skill3.md").exists());
    }

    #[test]
    fn test_skill_with_frontmatter() {
        let env = TestSkillEnvironment::new()
            .with_skill_full(
                "analyzer",
                "---\ntype: skill\nversion: 1.0\n---",
                "Analyze the code",
            )
            .build()
            .unwrap();

        let content = fs::read_to_string(env.path().join("analyzer.md")).unwrap();

        assert!(content.contains("---"));
        assert!(content.contains("type: skill"));
        assert!(content.contains("Analyze the code"));
    }

    #[test]
    fn test_automatic_cleanup() {
        let path = {
            let env = TestSkillEnvironment::new()
                .with_skill("temp-skill", "Temporary")
                .build()
                .unwrap();

            let path = env.path().to_path_buf();
            assert!(path.exists());
            path
            // env is dropped here
        };

        // After guard is dropped, directory should be cleaned up
        assert!(!path.exists(), "Temporary directory should be cleaned up");
    }

    #[test]
    fn test_empty_environment() {
        let env = TestSkillEnvironment::new().build().unwrap();

        // Directory exists but is empty
        assert!(env.path().exists());
        assert!(env.path().is_dir());

        let entries: Vec<_> = fs::read_dir(env.path()).unwrap().collect();
        assert_eq!(entries.len(), 0, "Should be empty directory");
    }
}
