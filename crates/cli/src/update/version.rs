//! Version detection and comparison module

use crate::update::error::UpdateError;
use serde::{Deserialize, Serialize};

/// Represents a semantic version (major.minor.patch)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Version {
    /// Create a new version from components
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Get the current binary version from compile-time environment
    pub fn current() -> Self {
        let version_str = env!("CARGO_PKG_VERSION");
        Version::parse(version_str).unwrap_or_else(|_| {
            // Fallback if parsing fails - this should not happen in normal builds
            Version::new(0, 0, 0)
        })
    }

    /// Parse a version string in the format "major.minor.patch"
    /// Handles optional 'v' prefix (e.g., "v1.2.3" or "1.2.3")
    pub fn parse(version_str: &str) -> Result<Self, UpdateError> {
        let version_str = version_str.trim_start_matches('v');

        let parts: Vec<&str> = version_str.split('.').collect();

        if parts.len() < 3 {
            return Err(UpdateError::VersionParseFailed(format!(
                "Expected 'major.minor.patch', got '{}'",
                version_str
            )));
        }

        let major = parts[0].parse::<u32>().map_err(|_| {
            UpdateError::VersionParseFailed(format!("Invalid major version: {}", parts[0]))
        })?;

        let minor = parts[1].parse::<u32>().map_err(|_| {
            UpdateError::VersionParseFailed(format!("Invalid minor version: {}", parts[1]))
        })?;

        // Handle patch version that might include pre-release or build metadata
        let patch_str = parts[2].split('-').next().unwrap_or(parts[2]);
        let patch_str = patch_str.split('+').next().unwrap_or(patch_str);

        let patch = patch_str.parse::<u32>().map_err(|_| {
            UpdateError::VersionParseFailed(format!("Invalid patch version: {}", patch_str))
        })?;

        Ok(Version {
            major,
            minor,
            patch,
        })
    }

    // Note: to_string() provided by Display trait implementation

    /// Compare two versions, returning true if self is greater than other
    pub fn is_greater_than(&self, other: &Version) -> bool {
        self > other
    }

    /// Compare two versions, returning true if self is less than other
    pub fn is_less_than(&self, other: &Version) -> bool {
        self < other
    }

    /// Check if this version is an update from the other (new version is greater)
    pub fn is_update_available(&self, current: &Version) -> bool {
        self > current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing_standard() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_parsing_with_v_prefix() {
        let version = Version::parse("v1.2.3").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_parsing_with_prerelease() {
        let version = Version::parse("1.2.3-alpha").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_parsing_with_metadata() {
        let version = Version::parse("1.2.3+build.1").unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_version_parsing_invalid() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.x").is_err());
        assert!(Version::parse("a.b.c").is_err());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 4);
        let v3 = Version::new(1, 3, 0);
        let v4 = Version::new(2, 0, 0);

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v3 < v4);
        assert!(v4 > v1);
    }

    #[test]
    fn test_version_equality() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 3);
        let v3 = Version::new(1, 2, 4);

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_is_greater_than() {
        let v1 = Version::new(1, 2, 3);
        let v2 = Version::new(1, 2, 2);

        assert!(v1.is_greater_than(&v2));
        assert!(!v2.is_greater_than(&v1));
    }

    #[test]
    fn test_is_update_available() {
        let current = Version::new(1, 0, 0);
        let new = Version::new(1, 1, 0);

        assert!(new.is_update_available(&current));
        assert!(!current.is_update_available(&new));
    }

    #[test]
    fn test_to_string() {
        let version = Version::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_current_version() {
        let current = Version::current();
        // Current version should exist (parsed from env!("CARGO_PKG_VERSION"))
        assert!(!current.to_string().is_empty());
    }
}
