use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DecisionStatus {
    #[default]
    Proposed,
    Accepted,
    Deprecated,
    Superseded,
}

impl std::fmt::Display for DecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionStatus::Proposed => write!(f, "proposed"),
            DecisionStatus::Accepted => write!(f, "accepted"),
            DecisionStatus::Deprecated => write!(f, "deprecated"),
            DecisionStatus::Superseded => write!(f, "superseded"),
        }
    }
}

impl std::str::FromStr for DecisionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "proposed" => Ok(DecisionStatus::Proposed),
            "accepted" => Ok(DecisionStatus::Accepted),
            "deprecated" => Ok(DecisionStatus::Deprecated),
            "superseded" => Ok(DecisionStatus::Superseded),
            _ => Err(format!("Invalid status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    #[serde(flatten)]
    pub base: EntityBase,
    pub status: DecisionStatus,
    pub context: Option<String>,
    pub consequences: Vec<String>,
    pub superseded_by: Option<String>,
}

impl Decision {
    pub fn new(title: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            status: DecisionStatus::default(),
            context: None,
            consequences: Vec::new(),
            superseded_by: None,
        }
    }
}
