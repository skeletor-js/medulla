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
