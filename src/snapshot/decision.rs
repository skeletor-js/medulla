// src/snapshot/decision.rs
//! Decision snapshot generation

use std::path::Path;

use serde::Serialize;

use crate::entity::Decision;
use crate::storage::LoroStore;
use crate::Result;

use super::{yaml_frontmatter, GeneratedFile};
use super::utils::{format_date, slugify, write_snapshot_file};

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
            body.push_str("\n");
        }
    }

    // Main content (the decision itself)
    if let Some(content) = &decision.base.content {
        if !content.is_empty() {
            if !body.is_empty() {
                body.push_str("\n## Decision\n\n");
            }
            body.push_str(content);
            body.push_str("\n");
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
