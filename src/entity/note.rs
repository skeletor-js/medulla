// src/entity/note.rs
use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    #[serde(flatten)]
    pub base: EntityBase,
    pub note_type: Option<String>,
}

impl Note {
    pub fn new(title: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            note_type: None,
        }
    }
}
