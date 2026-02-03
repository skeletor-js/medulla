// src/snapshot/note.rs
//! Note snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Note;
use crate::storage::LoroStore;
use crate::Result;

use super::{yaml_frontmatter, GeneratedFile};
use super::utils::{format_date, slugify, write_snapshot_file};

#[derive(Serialize)]
struct NoteFrontmatter {
    id: String,
    sequence: u32,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    note_type: Option<String>,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
}

impl NoteFrontmatter {
    fn from_note(note: &Note) -> Self {
        Self {
            id: note.base.id.to_string(),
            sequence: note.base.sequence_number,
            title: note.base.title.clone(),
            note_type: note.note_type.clone(),
            created: format_date(&note.base.created_at),
            updated: format_date(&note.base.updated_at),
            created_by: note.base.created_by.clone(),
            tags: note.base.tags.clone(),
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

/// Generate note snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let notes = store.list_notes()?;
    let mut generated = Vec::new();

    if notes.is_empty() {
        return Ok(generated);
    }

    // Sort by sequence number for consistent ordering
    let mut sorted_notes = notes;
    sorted_notes.sort_by_key(|n| n.base.sequence_number);

    let notes_dir = snapshot_dir.join("notes");
    let mut used_slugs = HashSet::new();

    for note in &sorted_notes {
        let frontmatter = NoteFrontmatter::from_note(note);
        let yaml = yaml_frontmatter(&frontmatter)?;

        // Content is just the body
        let body = note.base.content.as_deref().unwrap_or("");
        let content = format!("{}\n{}", yaml, body);

        let slug = slugify(&note.base.title);
        let filename = unique_filename(&slug, note.base.sequence_number, &mut used_slugs);
        let file_path = notes_dir.join(&filename);

        write_snapshot_file(&file_path, &content)?;

        generated.push(GeneratedFile {
            relative_path: format!("notes/{}", filename),
            entity_count: 1,
        });
    }

    Ok(generated)
}
