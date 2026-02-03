// src/snapshot/component.rs
//! Component snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Component;
use crate::storage::LoroStore;
use crate::Result;

use super::utils::{format_date, slugify, write_snapshot_file};
use super::{yaml_frontmatter, GeneratedFile};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::ComponentStatus;
    use tempfile::TempDir;

    fn mock_component(seq: u32, title: &str, status: ComponentStatus) -> Component {
        let mut component = Component::new(title.to_string(), seq);
        component.base.content = Some("Component description".to_string());
        component.base.tags = vec!["backend".to_string()];
        component.component_type = Some("service".to_string());
        component.status = status;
        component.owner = Some("Team A".to_string());
        component
    }

    #[test]
    fn test_component_frontmatter_all_fields() {
        let component = mock_component(1, "Auth Service", ComponentStatus::Active);
        let fm = ComponentFrontmatter::from_component(&component);

        assert_eq!(fm.title, "Auth Service");
        assert_eq!(fm.sequence, 1);
        assert_eq!(fm.component_type, Some("service".to_string()));
        assert_eq!(fm.status, "active");
        assert_eq!(fm.owner, Some("Team A".to_string()));
        assert_eq!(fm.tags, vec!["backend".to_string()]);
    }

    #[test]
    fn test_component_frontmatter_optional_fields() {
        let mut component = mock_component(1, "Simple", ComponentStatus::Active);
        component.component_type = None;
        component.owner = None;
        component.base.tags = vec![];

        let fm = ComponentFrontmatter::from_component(&component);

        assert!(fm.component_type.is_none());
        assert!(fm.owner.is_none());
        assert!(fm.tags.is_empty());
    }

    #[test]
    fn test_component_status_badge() {
        let active = mock_component(1, "Active", ComponentStatus::Active);
        let deprecated = mock_component(2, "Old", ComponentStatus::Deprecated);
        let planned = mock_component(3, "Future", ComponentStatus::Planned);

        assert_eq!(
            ComponentFrontmatter::from_component(&active).status,
            "active"
        );
        assert_eq!(
            ComponentFrontmatter::from_component(&deprecated).status,
            "deprecated"
        );
        assert_eq!(
            ComponentFrontmatter::from_component(&planned).status,
            "planned"
        );
    }

    #[test]
    fn test_generate_component_files() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let component = Component::new("API Gateway".to_string(), 1);
        store.add_component(&component).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.starts_with("components/"));
        assert!(files[0].relative_path.contains("api-gateway"));
    }

    #[test]
    fn test_component_file_content() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let mut component = Component::new("Database".to_string(), 1);
        component.base.content = Some("PostgreSQL database".to_string());
        component.owner = Some("DBA Team".to_string());
        store.add_component(&component).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        generate(&store, &snapshot_dir).unwrap();

        let file_path = snapshot_dir.join("components/database.md");
        let content = std::fs::read_to_string(&file_path).unwrap();

        assert!(content.starts_with("---\n"));
        assert!(content.contains("title: Database"));
        assert!(content.contains("status: active"));
        assert!(content.contains("owner: DBA Team"));
        assert!(content.contains("PostgreSQL database"));
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
        used.insert("my-component".to_string());

        let filename = unique_filename("my-component", 2, &mut used);

        assert_eq!(filename, "my-component-2.md");
    }
}
