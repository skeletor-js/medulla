// src/snapshot/mod.rs
//! Snapshot generation module
//!
//! Generates human-readable markdown snapshots of all entities.
//! These snapshots are derived views meant for browsing on GitHub.

mod decision;
mod task;
mod note;
mod prompt;
mod component;
mod link;
mod readme;
pub mod utils;

use std::path::Path;

use chrono::Utc;

use crate::storage::LoroStore;
use crate::Result;

pub use self::utils::{slugify, format_date, format_timestamp, short_uuid};

/// Statistics about generated snapshot
#[derive(Debug, Default)]
pub struct SnapshotStats {
    pub decisions: usize,
    pub tasks_total: usize,
    pub tasks_active: usize,
    pub tasks_completed: usize,
    pub notes: usize,
    pub prompts: usize,
    pub components: usize,
    pub links: usize,
    pub files_generated: Vec<String>,
}

impl SnapshotStats {
    /// Total number of entities
    pub fn total_entities(&self) -> usize {
        self.decisions + self.tasks_total + self.notes + self.prompts + self.components + self.links
    }
}

/// Result of generating a single snapshot file
pub struct GeneratedFile {
    pub relative_path: String,
    pub entity_count: usize,
}

/// Generate markdown snapshots for all entities
///
/// This will:
/// 1. Clear the existing snapshot directory
/// 2. Create fresh snapshots for all entity types
/// 3. Generate an index README.md
pub fn generate_snapshot(store: &LoroStore, snapshot_dir: &Path) -> Result<SnapshotStats> {
    let mut stats = SnapshotStats::default();

    // Clear and recreate directory structure
    utils::clear_snapshot_dir(snapshot_dir)?;
    utils::ensure_snapshot_dirs(snapshot_dir)?;

    // Generate snapshots for each entity type
    let decision_files = decision::generate(store, snapshot_dir)?;
    stats.decisions = decision_files.iter().map(|f| f.entity_count).sum();
    stats.files_generated.extend(decision_files.into_iter().map(|f| f.relative_path));

    let task_files = task::generate(store, snapshot_dir)?;
    for file in &task_files {
        if file.relative_path.contains("active") {
            stats.tasks_active = file.entity_count;
        } else if file.relative_path.contains("completed") {
            stats.tasks_completed = file.entity_count;
        }
    }
    stats.tasks_total = stats.tasks_active + stats.tasks_completed;
    stats.files_generated.extend(task_files.into_iter().map(|f| f.relative_path));

    let note_files = note::generate(store, snapshot_dir)?;
    stats.notes = note_files.iter().map(|f| f.entity_count).sum();
    stats.files_generated.extend(note_files.into_iter().map(|f| f.relative_path));

    let prompt_files = prompt::generate(store, snapshot_dir)?;
    stats.prompts = prompt_files.iter().map(|f| f.entity_count).sum();
    stats.files_generated.extend(prompt_files.into_iter().map(|f| f.relative_path));

    let component_files = component::generate(store, snapshot_dir)?;
    stats.components = component_files.iter().map(|f| f.entity_count).sum();
    stats.files_generated.extend(component_files.into_iter().map(|f| f.relative_path));

    let link_files = link::generate(store, snapshot_dir)?;
    stats.links = link_files.iter().map(|f| f.entity_count).sum();
    stats.files_generated.extend(link_files.into_iter().map(|f| f.relative_path));

    // Generate README index (must be last to have all stats)
    readme::generate(store, snapshot_dir, &stats)?;
    stats.files_generated.push("README.md".to_string());

    Ok(stats)
}

/// Generate YAML frontmatter block
pub fn yaml_frontmatter<T: serde::Serialize>(data: &T) -> Result<String> {
    let yaml = serde_yaml::to_string(data)
        .map_err(|e| crate::error::MedullaError::Storage(format!("YAML serialization failed: {}", e)))?;
    Ok(format!("---\n{}---\n", yaml))
}

/// Get current timestamp for "last updated" footers
pub fn current_timestamp() -> String {
    format_timestamp(&Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_yaml_frontmatter_format() {
        #[derive(serde::Serialize)]
        struct TestFrontmatter {
            title: String,
            status: String,
        }

        let fm = TestFrontmatter {
            title: "Test".to_string(),
            status: "accepted".to_string(),
        };

        let result = yaml_frontmatter(&fm).unwrap();
        assert!(result.starts_with("---\n"));
        assert!(result.ends_with("---\n"));
        assert!(result.contains("title: Test"));
        assert!(result.contains("status: accepted"));
    }

    #[test]
    fn test_current_timestamp_format() {
        let ts = current_timestamp();
        // Should be in format "YYYY-MM-DD HH:MM:SS UTC"
        assert!(ts.contains("UTC"));
        assert!(ts.len() > 20);
    }

    #[test]
    fn test_snapshot_stats_total_entities() {
        let stats = SnapshotStats {
            decisions: 5,
            tasks_total: 10,
            tasks_active: 7,
            tasks_completed: 3,
            notes: 3,
            prompts: 2,
            components: 1,
            links: 4,
            files_generated: vec![],
        };

        // Total should be decisions + tasks_total + notes + prompts + components + links
        assert_eq!(stats.total_entities(), 5 + 10 + 3 + 2 + 1 + 4);
    }

    #[test]
    fn test_snapshot_stats_default() {
        let stats = SnapshotStats::default();
        assert_eq!(stats.total_entities(), 0);
        assert!(stats.files_generated.is_empty());
    }

    #[test]
    fn test_generate_snapshot_creates_directories() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        // Initialize a minimal store
        let store = LoroStore::init(&medulla_dir).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        let _stats = generate_snapshot(&store, &snapshot_dir).unwrap();

        // Verify directories were created
        assert!(snapshot_dir.exists());
        assert!(snapshot_dir.join("decisions").exists());
        assert!(snapshot_dir.join("tasks").exists());
        assert!(snapshot_dir.join("notes").exists());
        assert!(snapshot_dir.join("prompts").exists());
        assert!(snapshot_dir.join("components").exists());
        assert!(snapshot_dir.join("links").exists());
        assert!(snapshot_dir.join("README.md").exists());
    }

    #[test]
    fn test_generate_snapshot_empty_store() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = LoroStore::init(&medulla_dir).unwrap();
        let snapshot_dir = medulla_dir.join("snapshot");

        let stats = generate_snapshot(&store, &snapshot_dir).unwrap();

        assert_eq!(stats.decisions, 0);
        assert_eq!(stats.tasks_total, 0);
        assert_eq!(stats.notes, 0);
        assert_eq!(stats.prompts, 0);
        assert_eq!(stats.components, 0);
        assert_eq!(stats.links, 0);
        // README.md should still be generated
        assert!(stats.files_generated.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_generate_snapshot_clears_existing() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = LoroStore::init(&medulla_dir).unwrap();
        let snapshot_dir = medulla_dir.join("snapshot");

        // Create a stale file
        std::fs::create_dir_all(&snapshot_dir).unwrap();
        std::fs::write(snapshot_dir.join("stale.md"), "old content").unwrap();
        assert!(snapshot_dir.join("stale.md").exists());

        // Generate snapshot
        generate_snapshot(&store, &snapshot_dir).unwrap();

        // Stale file should be gone
        assert!(!snapshot_dir.join("stale.md").exists());
    }
}
