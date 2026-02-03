// src/snapshot/utils.rs
//! Utility functions for snapshot generation

use std::fs;
use std::path::Path;

use crate::Result;

/// Convert a title to a URL-safe slug
///
/// - Converts to lowercase
/// - Replaces spaces and special chars with hyphens
/// - Removes consecutive hyphens
/// - Trims leading/trailing hyphens
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut last_was_hyphen = true; // Start true to trim leading hyphens

    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            slug.push('-');
            last_was_hyphen = true;
        }
    }

    // Trim trailing hyphen
    if slug.ends_with('-') {
        slug.pop();
    }

    // Ensure non-empty slug
    if slug.is_empty() {
        slug = "untitled".to_string();
    }

    slug
}

/// Ensure the snapshot directory structure exists
pub fn ensure_snapshot_dirs(snapshot_dir: &Path) -> Result<()> {
    let subdirs = [
        "decisions",
        "tasks",
        "notes",
        "prompts",
        "components",
        "links",
    ];

    for subdir in &subdirs {
        fs::create_dir_all(snapshot_dir.join(subdir))?;
    }

    Ok(())
}

/// Remove all files from the snapshot directory (but keep the structure)
pub fn clear_snapshot_dir(snapshot_dir: &Path) -> Result<()> {
    if !snapshot_dir.exists() {
        return Ok(());
    }

    // Remove and recreate to ensure clean state
    fs::remove_dir_all(snapshot_dir)?;
    Ok(())
}

/// Write content to a file, creating parent directories if needed
pub fn write_snapshot_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Format a DateTime as YYYY-MM-DD for frontmatter
pub fn format_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d").to_string()
}

/// Format a DateTime as full ISO timestamp
pub fn format_timestamp(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Shorten a UUID for display (first 7 chars)
pub fn short_uuid(id: &uuid::Uuid) -> String {
    id.to_string()[..7].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Use PostgreSQL"), "use-postgresql");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("API v2.0 (beta)"), "api-v2-0-beta");
    }

    #[test]
    fn test_slugify_consecutive_specials() {
        assert_eq!(slugify("Hello   World"), "hello-world");
        assert_eq!(slugify("---test---"), "test");
    }

    #[test]
    fn test_slugify_unicode() {
        assert_eq!(slugify("Café au lait"), "caf-au-lait");
        assert_eq!(slugify("日本語"), "untitled"); // Non-ASCII only
    }

    #[test]
    fn test_slugify_empty() {
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("---"), "untitled");
    }

    #[test]
    fn test_slugify_numbers() {
        assert_eq!(slugify("Task 123"), "task-123");
        assert_eq!(slugify("2024-01-15 Meeting"), "2024-01-15-meeting");
    }
}
