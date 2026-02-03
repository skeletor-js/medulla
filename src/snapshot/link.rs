// src/snapshot/link.rs
//! Link snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Link;
use crate::storage::LoroStore;
use crate::Result;

use super::utils::{format_date, slugify, write_snapshot_file};
use super::{yaml_frontmatter, GeneratedFile};

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mock_link(seq: u32, title: &str, url: &str) -> Link {
        let mut link = Link::new(title.to_string(), url.to_string(), seq);
        link.base.content = Some("Link description".to_string());
        link.base.tags = vec!["docs".to_string()];
        link.link_type = Some("documentation".to_string());
        link
    }

    #[test]
    fn test_link_frontmatter_all_fields() {
        let link = mock_link(1, "Rust Docs", "https://doc.rust-lang.org");
        let fm = LinkFrontmatter::from_link(&link);

        assert_eq!(fm.title, "Rust Docs");
        assert_eq!(fm.sequence, 1);
        assert_eq!(fm.url, "https://doc.rust-lang.org");
        assert_eq!(fm.link_type, Some("documentation".to_string()));
        assert_eq!(fm.tags, vec!["docs".to_string()]);
    }

    #[test]
    fn test_link_frontmatter_url_required() {
        let link = mock_link(1, "Test", "https://example.com");
        let fm = LinkFrontmatter::from_link(&link);

        assert_eq!(fm.url, "https://example.com");
    }

    #[test]
    fn test_link_frontmatter_optional_type() {
        let mut link = mock_link(1, "Simple Link", "https://example.com");
        link.link_type = None;

        let fm = LinkFrontmatter::from_link(&link);

        assert!(fm.link_type.is_none());
    }

    #[test]
    fn test_link_type_badge() {
        let doc_link = mock_link(1, "Docs", "https://docs.example.com");
        let mut api_link = mock_link(2, "API", "https://api.example.com");
        api_link.link_type = Some("api".to_string());

        let fm1 = LinkFrontmatter::from_link(&doc_link);
        let fm2 = LinkFrontmatter::from_link(&api_link);

        assert_eq!(fm1.link_type, Some("documentation".to_string()));
        assert_eq!(fm2.link_type, Some("api".to_string()));
    }

    #[test]
    fn test_generate_link_files() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let link = Link::new(
            "GitHub Repo".to_string(),
            "https://github.com/example/repo".to_string(),
            1,
        );
        store.add_link(&link).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.starts_with("links/"));
        assert!(files[0].relative_path.contains("github-repo"));
    }

    #[test]
    fn test_link_file_content() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let mut link = Link::new(
            "Rust Homepage".to_string(),
            "https://www.rust-lang.org".to_string(),
            1,
        );
        link.base.content = Some("The Rust programming language".to_string());
        link.link_type = Some("homepage".to_string());
        store.add_link(&link).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        generate(&store, &snapshot_dir).unwrap();

        let file_path = snapshot_dir.join("links/rust-homepage.md");
        let content = std::fs::read_to_string(&file_path).unwrap();

        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: Rust Homepage"));
        assert!(content.contains("url: https://www.rust-lang.org"));
        assert!(content.contains("type: homepage"));
        assert!(content.contains("The Rust programming language"));
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
    fn test_unique_filename_collision() {
        let mut used = HashSet::new();
        used.insert("my-link".to_string());

        let filename = unique_filename("my-link", 2, &mut used);

        assert_eq!(filename, "my-link-2.md");
    }
}
