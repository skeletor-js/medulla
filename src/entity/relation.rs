use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Relation types supported by the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Task implements this decision
    Implements,
    /// Blocking dependency between tasks
    Blocks,
    /// New decision replaces old one
    Supersedes,
    /// General reference
    References,
    /// Task belongs to a component
    BelongsTo,
    /// Note documents a component
    Documents,
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::Implements => write!(f, "implements"),
            RelationType::Blocks => write!(f, "blocks"),
            RelationType::Supersedes => write!(f, "supersedes"),
            RelationType::References => write!(f, "references"),
            RelationType::BelongsTo => write!(f, "belongs_to"),
            RelationType::Documents => write!(f, "documents"),
        }
    }
}

impl std::str::FromStr for RelationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "implements" => Ok(RelationType::Implements),
            "blocks" => Ok(RelationType::Blocks),
            "supersedes" => Ok(RelationType::Supersedes),
            "references" => Ok(RelationType::References),
            "belongs_to" | "belongsto" => Ok(RelationType::BelongsTo),
            "documents" => Ok(RelationType::Documents),
            _ => Err(format!("Unknown relation type: {}", s)),
        }
    }
}

/// A relation between two entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Source entity UUID
    pub source_id: Uuid,
    /// Source entity type (denormalized for query filtering)
    pub source_type: String,
    /// Target entity UUID
    pub target_id: Uuid,
    /// Target entity type (denormalized for query filtering)
    pub target_type: String,
    /// Type of relation
    pub relation_type: RelationType,
    /// When the relation was created
    pub created_at: DateTime<Utc>,
    /// Who created the relation
    pub created_by: Option<String>,
    /// Additional properties (optional metadata)
    #[serde(default)]
    pub properties: HashMap<String, String>,
}

impl Relation {
    /// Create a new relation
    pub fn new(
        source_id: Uuid,
        source_type: String,
        target_id: Uuid,
        target_type: String,
        relation_type: RelationType,
    ) -> Self {
        Self {
            source_id,
            source_type,
            target_id,
            target_type,
            relation_type,
            created_at: Utc::now(),
            created_by: None,
            properties: HashMap::new(),
        }
    }

    /// Generate the composite key for this relation
    pub fn composite_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.source_id, self.relation_type, self.target_id
        )
    }
}
