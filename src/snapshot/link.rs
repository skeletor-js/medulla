// src/snapshot/link.rs
//! Link snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Link;
use crate::storage::LoroStore;
use crate::Result;

use super::{yaml_frontmatter, GeneratedFile};
use super::utils::{format_date, slugify, write_snapshot_file};

#[derive(Serialize)]
struct LinkFrontmatter {
    id: String,
    sequence: u32,
    title: String,
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    link_type: Option<String>,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

impl LinkFrontmatter {
    fn from_link(link: &Link) -> Self {
        Self {
            id: link.base.id.to_string(),
            sequence: link.base.sequence_number,
            title: link.base.title.clone(),
            url: link.url.clone(),
            link_type: link.link_type.clone(),
            created: format_date(&link.base.created_at),
            updated: format_date(&link.base.updated_at),
            created_by: link.base.created_by.clone(),
            tags: link.base.tags.clone(),
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

/// Generate link snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let links = store.list_links()?;
    let mut generated = Vec::new();

    if links.is_empty() {
        return Ok(generated);
    }

    // Sort by sequence number for consistent ordering
    let mut sorted_links = links;
    sorted_links.sort_by_key(|l| l.base.sequence_number);

    let links_dir = snapshot_dir.join("links");
    let mut used_slugs = HashSet::new();

    for link in &sorted_links {
        let frontmatter = LinkFrontmatter::from_link(link);
        let yaml = yaml_frontmatter(&frontmatter)?;

        // Content is just the body
        let body = link.base.content.as_deref().unwrap_or("");
        let content = format!("{}\n{}", yaml, body);

        let slug = slugify(&link.base.title);
        let filename = unique_filename(&slug, link.base.sequence_number, &mut used_slugs);
        let file_path = links_dir.join(&filename);

        write_snapshot_file(&file_path, &content)?;

        generated.push(GeneratedFile {
            relative_path: format!("links/{}", filename),
            entity_count: 1,
        });
    }

    Ok(generated)
}
