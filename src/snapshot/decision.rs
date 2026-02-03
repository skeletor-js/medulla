// src/snapshot/decision.rs
//! Decision snapshot generation

use std::path::Path;

use serde::Serialize;

use crate::entity::Decision;
use crate::storage::LoroStore;
use crate::Result;

use super::utils::{format_date, slugify, write_snapshot_file};
use super::{yaml_frontmatter, GeneratedFile};

#[derive(Serialize)]
struct DecisionFrontmatter {
    id: String,
    sequence: u32,
    title: String,
    status: String,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    superseded_by: Option<String>,
}

impl DecisionFrontmatter {
    fn from_decision(decision: &Decision) -> Self {
        Self {
            id: decision.base.id.to_string(),
            sequence: decision.base.sequence_number,
            title: decision.base.title.clone(),
            status: decision.status.to_string(),
            created: format_date(&decision.base.created_at),
            updated: format_date(&decision.base.updated_at),
            created_by: decision.base.created_by.clone(),
            tags: decision.base.tags.clone(),
            superseded_by: decision.superseded_by.clone(),
        }
    }
}

/// Generate markdown body for a decision
fn generate_body(decision: &Decision) -> String {
    let mut body = String::new();

    // Context section (if present)
    if let Some(context) = &decision.context {
        if !context.is_empty() {
            body.push_str("\n## Context\n\n");
            body.push_str(context);
            body.push('\n');
        }
    }

    // Main content (the decision itself)
    if let Some(content) = &decision.base.content {
        if !content.is_empty() {
            if !body.is_empty() {
                body.push_str("\n## Decision\n\n");
            }
            body.push_str(content);
            body.push('\n');
        }
    }

    // Consequences section (if present)
    if !decision.consequences.is_empty() {
        body.push_str("\n## Consequences\n\n");
        for consequence in &decision.consequences {
            body.push_str(&format!("- {}\n", consequence));
        }
    }

    body
}

/// Generate decision snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let decisions = store.list_decisions()?;
    let mut generated = Vec::new();

    if decisions.is_empty() {
        return Ok(generated);
    }

    // Sort by sequence number for consistent ordering
    let mut sorted_decisions = decisions;
    sorted_decisions.sort_by_key(|d| d.base.sequence_number);

    let decisions_dir = snapshot_dir.join("decisions");

    for decision in &sorted_decisions {
        let frontmatter = DecisionFrontmatter::from_decision(decision);
        let yaml = yaml_frontmatter(&frontmatter)?;
        let body = generate_body(decision);

        let content = format!("{}{}", yaml, body);

        // Filename: {sequence:03}-{slug}.md
        let slug = slugify(&decision.base.title);
        let filename = format!("{:03}-{}.md", decision.base.sequence_number, slug);
        let file_path = decisions_dir.join(&filename);

        write_snapshot_file(&file_path, &content)?;

        generated.push(GeneratedFile {
            relative_path: format!("decisions/{}", filename),
            entity_count: 1,
        });
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::DecisionStatus;
    use tempfile::TempDir;

    fn mock_decision(seq: u32, title: &str, status: DecisionStatus) -> Decision {
        let mut decision = Decision::new(title.to_string(), seq);
        decision.status = status;
        decision.base.tags = vec!["test".to_string()];
        decision.context = Some("Test context".to_string());
        decision.consequences = vec!["Consequence 1".to_string(), "Consequence 2".to_string()];
        decision
    }

    #[test]
    fn test_decision_frontmatter_all_fields() {
        let decision = mock_decision(1, "Use PostgreSQL", DecisionStatus::Accepted);
        let fm = DecisionFrontmatter::from_decision(&decision);

        assert_eq!(fm.title, "Use PostgreSQL");
        assert_eq!(fm.status, "accepted");
        assert_eq!(fm.sequence, 1);
        assert!(!fm.id.is_empty());
        assert_eq!(fm.tags, vec!["test".to_string()]);
    }

    #[test]
    fn test_decision_frontmatter_optional_fields() {
        let mut decision = mock_decision(1, "Test", DecisionStatus::Proposed);
        decision.base.created_by = None;
        decision.base.tags = vec![];
        decision.superseded_by = None;

        let fm = DecisionFrontmatter::from_decision(&decision);

        assert!(fm.created_by.is_none());
        assert!(fm.tags.is_empty());
        assert!(fm.superseded_by.is_none());
    }

    #[test]
    fn test_decision_frontmatter_superseded_by() {
        let mut decision = mock_decision(1, "Old Decision", DecisionStatus::Superseded);
        decision.superseded_by = Some("abc123".to_string());

        let fm = DecisionFrontmatter::from_decision(&decision);

        assert_eq!(fm.superseded_by, Some("abc123".to_string()));
    }

    #[test]
    fn test_generate_body_with_context() {
        let decision = mock_decision(1, "Test", DecisionStatus::Accepted);
        let body = generate_body(&decision);

        assert!(body.contains("## Context"));
        assert!(body.contains("Test context"));
    }

    #[test]
    fn test_generate_body_with_consequences() {
        let decision = mock_decision(1, "Test", DecisionStatus::Accepted);
        let body = generate_body(&decision);

        assert!(body.contains("## Consequences"));
        assert!(body.contains("- Consequence 1"));
        assert!(body.contains("- Consequence 2"));
    }

    #[test]
    fn test_generate_body_with_content() {
        let mut decision = mock_decision(1, "Test", DecisionStatus::Accepted);
        decision.base.content = Some("The actual decision content".to_string());
        let body = generate_body(&decision);

        assert!(body.contains("## Decision"));
        assert!(body.contains("The actual decision content"));
    }

    #[test]
    fn test_generate_body_empty_context_skipped() {
        let mut decision = mock_decision(1, "Test", DecisionStatus::Accepted);
        decision.context = Some("".to_string());
        let body = generate_body(&decision);

        assert!(!body.contains("## Context"));
    }

    #[test]
    fn test_decision_filename_format() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let mut decision = Decision::new("Use PostgreSQL".to_string(), 1);
        decision.status = DecisionStatus::Accepted;
        store.add_decision(&decision).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 1);
        // Format: 001-use-postgresql.md
        assert!(files[0].relative_path.starts_with("decisions/001-"));
        assert!(files[0].relative_path.ends_with(".md"));
        assert!(files[0].relative_path.contains("use-postgresql"));
    }

    #[test]
    fn test_decision_file_content() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let mut decision = Decision::new("Test Decision".to_string(), 1);
        decision.status = DecisionStatus::Accepted;
        store.add_decision(&decision).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        generate(&store, &snapshot_dir).unwrap();

        let slug = slugify(&decision.base.title);
        let file_path = snapshot_dir.join(format!(
            "decisions/{:03}-{}.md",
            decision.base.sequence_number, slug
        ));

        let content = std::fs::read_to_string(&file_path).unwrap();

        // Verify YAML frontmatter
        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: Test Decision"));
        assert!(content.contains("status: accepted"));
        assert!(content.contains("---\n"));
    }

    #[test]
    fn test_generate_empty_store() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn test_generate_multiple_decisions_sorted() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();

        let mut d1 = Decision::new("First Decision".to_string(), 1);
        d1.status = DecisionStatus::Accepted;
        store.add_decision(&d1).unwrap();

        let mut d2 = Decision::new("Second Decision".to_string(), 2);
        d2.status = DecisionStatus::Proposed;
        store.add_decision(&d2).unwrap();

        let mut d3 = Decision::new("Third Decision".to_string(), 3);
        d3.status = DecisionStatus::Deprecated;
        store.add_decision(&d3).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 3);
        // Files should be numbered in order
        assert!(files[0].relative_path.contains("001-"));
        assert!(files[1].relative_path.contains("002-"));
        assert!(files[2].relative_path.contains("003-"));
    }
}
