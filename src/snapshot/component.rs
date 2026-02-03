// src/snapshot/component.rs
//! Component snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Component;
use crate::storage::LoroStore;
use crate::Result;

use super::{yaml_frontmatter, GeneratedFile};
use super::utils::{format_date, slugify, write_snapshot_file};

#[derive(Serialize)]
struct ComponentFrontmatter {
    id: String,
    sequence: u32,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    component_type: Option<String>,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

impl ComponentFrontmatter {
    fn from_component(component: &Component) -> Self {
        Self {
            id: component.base.id.to_string(),
            sequence: component.base.sequence_number,
            title: component.base.title.clone(),
            component_type: component.component_type.clone(),
            status: component.status.to_string(),
            owner: component.owner.clone(),
            created: format_date(&component.base.created_at),
            updated: format_date(&component.base.updated_at),
            created_by: component.base.created_by.clone(),
            tags: component.base.tags.clone(),
        }
    }
}

/// Generate a unique filename, handling collisions
fn unique_filename(base_slug: &str, sequence: u32, used_slugs: &mut HashSet<String>) -> String {
    let candidate = base_slug.to_string();

    if used_slugs.insert(candidate.clone()) {
        format!("{}.md", candidate)
    } else {
        // Collision: append sequence number
        let unique = format!("{}-{}", base_slug, sequence);
        used_slugs.insert(unique.clone());
        format!("{}.md", unique)
    }
}

/// Generate component snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let components = store.list_components()?;
    let mut generated = Vec::new();

    if components.is_empty() {
        return Ok(generated);
    }

    // Sort by sequence number for consistent ordering
    let mut sorted_components = components;
    sorted_components.sort_by_key(|c| c.base.sequence_number);

    let components_dir = snapshot_dir.join("components");
    let mut used_slugs = HashSet::new();

    for component in &sorted_components {
        let frontmatter = ComponentFrontmatter::from_component(component);
        let yaml = yaml_frontmatter(&frontmatter)?;

        // Content is just the body
        let body = component.base.content.as_deref().unwrap_or("");
        let content = format!("{}\n{}", yaml, body);

        let slug = slugify(&component.base.title);
        let filename = unique_filename(&slug, component.base.sequence_number, &mut used_slugs);
        let file_path = components_dir.join(&filename);

        write_snapshot_file(&file_path, &content)?;

        generated.push(GeneratedFile {
            relative_path: format!("components/{}", filename),
            entity_count: 1,
        });
    }

    Ok(generated)
}
