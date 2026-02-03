// src/snapshot/prompt.rs
//! Prompt snapshot generation

use std::collections::HashSet;
use std::path::Path;

use serde::Serialize;

use crate::entity::Prompt;
use crate::storage::LoroStore;
use crate::Result;

use super::{yaml_frontmatter, GeneratedFile};
use super::utils::{format_date, slugify, write_snapshot_file};

#[derive(Serialize)]
struct PromptFrontmatter {
    id: String,
    sequence: u32,
    title: String,
    created: String,
    updated: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_by: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    variables: Vec<String>,
}

impl PromptFrontmatter {
    fn from_prompt(prompt: &Prompt) -> Self {
        Self {
            id: prompt.base.id.to_string(),
            sequence: prompt.base.sequence_number,
            title: prompt.base.title.clone(),
            created: format_date(&prompt.base.created_at),
            updated: format_date(&prompt.base.updated_at),
            created_by: prompt.base.created_by.clone(),
            tags: prompt.base.tags.clone(),
            variables: prompt.variables.clone(),
        }
    }
}

/// Generate markdown body for a prompt
fn generate_body(prompt: &Prompt) -> String {
    let mut body = String::new();

    // Description from content
    if let Some(content) = &prompt.base.content {
        if !content.is_empty() {
            body.push_str(content);
            body.push_str("\n");
        }
    }

    // Template section
    if let Some(template) = &prompt.template {
        if !template.is_empty() {
            body.push_str("\n## Template\n\n");
            body.push_str(template);
            body.push_str("\n");
        }
    }

    // Output schema section
    if let Some(schema) = &prompt.output_schema {
        if !schema.is_empty() {
            body.push_str("\n## Output Schema\n\n");
            body.push_str("```json\n");
            body.push_str(schema);
            if !schema.ends_with('\n') {
                body.push_str("\n");
            }
            body.push_str("```\n");
        }
    }

    body
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

/// Generate prompt snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let prompts = store.list_prompts()?;
    let mut generated = Vec::new();

    if prompts.is_empty() {
        return Ok(generated);
    }

    // Sort by sequence number for consistent ordering
    let mut sorted_prompts = prompts;
    sorted_prompts.sort_by_key(|p| p.base.sequence_number);

    let prompts_dir = snapshot_dir.join("prompts");
    let mut used_slugs = HashSet::new();

    for prompt in &sorted_prompts {
        let frontmatter = PromptFrontmatter::from_prompt(prompt);
        let yaml = yaml_frontmatter(&frontmatter)?;
        let body = generate_body(prompt);

        let content = format!("{}{}", yaml, body);

        let slug = slugify(&prompt.base.title);
        let filename = unique_filename(&slug, prompt.base.sequence_number, &mut used_slugs);
        let file_path = prompts_dir.join(&filename);

        write_snapshot_file(&file_path, &content)?;

        generated.push(GeneratedFile {
            relative_path: format!("prompts/{}", filename),
            entity_count: 1,
        });
    }

    Ok(generated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn mock_prompt(seq: u32, title: &str) -> Prompt {
        let mut prompt = Prompt::new(title.to_string(), seq);
        prompt.base.content = Some("Prompt description".to_string());
        prompt.template = Some("Hello {{name}}".to_string());
        prompt.variables = vec!["name".to_string()];
        prompt
    }

    #[test]
    fn test_prompt_frontmatter_all_fields() {
        let prompt = mock_prompt(1, "Greeting Prompt");
        let fm = PromptFrontmatter::from_prompt(&prompt);

        assert_eq!(fm.title, "Greeting Prompt");
        assert_eq!(fm.sequence, 1);
        assert_eq!(fm.variables, vec!["name".to_string()]);
    }

    #[test]
    fn test_prompt_frontmatter_empty_variables() {
        let mut prompt = mock_prompt(1, "Simple Prompt");
        prompt.variables = vec![];

        let fm = PromptFrontmatter::from_prompt(&prompt);

        assert!(fm.variables.is_empty());
    }

    #[test]
    fn test_generate_body_with_template() {
        let prompt = mock_prompt(1, "Test");
        let body = generate_body(&prompt);

        assert!(body.contains("## Template"));
        assert!(body.contains("Hello {{name}}"));
    }

    #[test]
    fn test_generate_body_with_content() {
        let prompt = mock_prompt(1, "Test");
        let body = generate_body(&prompt);

        assert!(body.contains("Prompt description"));
    }

    #[test]
    fn test_generate_body_with_output_schema() {
        let mut prompt = mock_prompt(1, "Test");
        prompt.output_schema = Some(r#"{"type": "string"}"#.to_string());

        let body = generate_body(&prompt);

        assert!(body.contains("## Output Schema"));
        assert!(body.contains("```json"));
        assert!(body.contains(r#"{"type": "string"}"#));
        assert!(body.contains("```"));
    }

    #[test]
    fn test_generate_body_empty_template_skipped() {
        let mut prompt = mock_prompt(1, "Test");
        prompt.template = Some("".to_string());

        let body = generate_body(&prompt);

        assert!(!body.contains("## Template"));
    }

    #[test]
    fn test_generate_prompt_files() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();
        let prompt = Prompt::new("Code Review".to_string(), 1);
        store.add_prompt(&prompt).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].relative_path.starts_with("prompts/"));
        assert!(files[0].relative_path.contains("code-review"));
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
        used.insert("my-prompt".to_string());

        let filename = unique_filename("my-prompt", 2, &mut used);

        assert_eq!(filename, "my-prompt-2.md");
    }
}
