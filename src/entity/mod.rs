mod decision;

pub use decision::Decision;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base fields shared by all entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBase {
    pub id: Uuid,
    pub title: String,
    pub content: Option<String>,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<String>,
    pub sequence_number: u32,
}

impl EntityBase {
    pub fn new(title: String, sequence_number: u32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            content: None,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            created_by: None,
            sequence_number,
        }
    }
}
