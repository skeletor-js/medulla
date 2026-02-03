// src/snapshot/note.rs
//! Note snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Note;
use crate::storage::LoroStore;
use crate::Result;

use super::utils::{format_date, slugify, write_snapshot_file};
use super::{yaml_frontmatter, GeneratedFile};

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mock_note(seq: u32, title: &str) -> Note {
        let mut note = Note::new(title.to_string(), seq);
        note.base.content = Some("Note content here".to_string());
        note.base.tags = vec!["test".to_string()];
        note.note_type = Some("meeting".to_string());
        note
    }

    #[test]
    fn test_note_frontmatter_all_fields() {
        let note = mock_note(1, "Project Meeting Notes");
        let fm = NoteFrontmatter::from_note(&note);

        assert_eq!(fm.title, "Project Meeting Notes");
        assert_eq!(fm.sequence, 1);
        assert_eq!(fm.note_type, Some("meeting".to_string()));
        assert_eq!(fm.tags, vec!["test".to_string()]);
    }

    #[test]
    fn test_note_frontmatter_optional_fields() {
        let mut note = mock_note(1, "Simple Note");
        note.note_type = None;
        note.base.tags = vec![];
        note.base.created_by = None;

        let fm = NoteFrontmatter::from_note(&note);

        assert!(fm.note_type.is_none());
        assert!(fm.tags.is_empty());
        assert!(fm.created_by.is_none());
    }

    #[test]
    fn test_unique_filename_no_collision() {
        let mut used = HashSet::new();
        let filename = unique_filename("my-note", 1, &mut used);

        assert_eq!(filename, "my-note.md");
        assert!(used.contains("my-note"));
    }

    #[test]
    fn test_unique_filename_with_collision() {
        let mut used = HashSet::new();
        used.insert("my-note".to_string());

        let filename = unique_filename("my-note", 2, &mut used);

        assert_eq!(filename, "my-note-2.md");
        assert!(used.contains("my-note-2"));
    }

    #[test]
    fn test_generate_note_files() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let note = Note::new("Test Note".to_string(), 1);
        store.add_note(&note).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.starts_with("notes/"));
        assert!(files[0].relative_path.ends_with(".md"));
    }

    #[test]
    fn test_note_file_content() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let mut note = Note::new("My Note".to_string(), 1);
        note.base.content = Some("This is the note body".to_string());
        store.add_note(&note).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        generate(&store, &snapshot_dir).unwrap();

        let file_path = snapshot_dir.join("notes/my-note.md");
        let content = std::fs::read_to_string(&file_path).unwrap();

        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: My Note"));
        assert!(content.contains("This is the note body"));
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
    fn test_slug_collision_handling() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        // Add two notes with the same title
        let note1 = Note::new("Same Title".to_string(), 1);
        let note2 = Note::new("Same Title".to_string(), 2);
        store.add_note(&note1).unwrap();
        store.add_note(&note2).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 2);
        // One should be "same-title.md", the other "same-title-2.md"
        let paths: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert!(paths.contains(&"notes/same-title.md"));
        assert!(paths.contains(&"notes/same-title-2.md"));
    }
}
