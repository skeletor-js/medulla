// src/entity/link.rs
use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    #[serde(flatten)]
    pub base: EntityBase,
    /// The URL this link points to
    pub url: String,
    /// Type of link (e.g., "documentation", "issue", "pr", "reference")
    pub link_type: Option<String>,
}

impl Link {
    pub fn new(title: String, url: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            url,
            link_type: None,
        }
    }
}
