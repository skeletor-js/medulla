// src/entity/prompt.rs
use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    #[serde(flatten)]
    pub base: EntityBase,
    /// The prompt template text (may include {{variable}} placeholders)
    pub template: Option<String>,
    /// Variable names expected by this prompt
    pub variables: Vec<String>,
    /// Optional JSON schema for expected output
    pub output_schema: Option<String>,
}

impl Prompt {
    pub fn new(title: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            template: None,
            variables: Vec::new(),
            output_schema: None,
        }
    }
}
