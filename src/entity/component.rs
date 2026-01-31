// src/entity/component.rs
use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ComponentStatus {
    #[default]
    Active,
    Deprecated,
    Planned,
}

impl std::fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentStatus::Active => write!(f, "active"),
            ComponentStatus::Deprecated => write!(f, "deprecated"),
            ComponentStatus::Planned => write!(f, "planned"),
        }
    }
}

impl std::str::FromStr for ComponentStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(ComponentStatus::Active),
            "deprecated" => Ok(ComponentStatus::Deprecated),
            "planned" => Ok(ComponentStatus::Planned),
            _ => Err(format!("Invalid component status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    #[serde(flatten)]
    pub base: EntityBase,
    pub component_type: Option<String>,
    pub status: ComponentStatus,
    pub owner: Option<String>,
}

impl Component {
    pub fn new(title: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            component_type: None,
            status: ComponentStatus::default(),
            owner: None,
        }
    }
}
