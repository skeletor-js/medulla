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
