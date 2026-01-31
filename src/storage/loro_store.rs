use std::fs;
use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};

use crate::entity::Decision;
use crate::error::{MedullaError, Result};

const MEDULLA_DIR: &str = ".medulla";
const LORO_DB: &str = "loro.db";

pub struct LoroStore {
    doc: LoroDoc,
    path: PathBuf,
}

impl LoroStore {
    /// Initialize a new medulla project
    pub fn init(root: &Path) -> Result<Self> {
        let medulla_dir = root.join(MEDULLA_DIR);

        if medulla_dir.exists() {
            return Err(MedullaError::AlreadyInitialized);
        }

        fs::create_dir_all(&medulla_dir)?;

        let doc = LoroDoc::new();
        let path = medulla_dir.join(LORO_DB);

        let store = Self { doc, path };
        store.save()?;

        Ok(store)
    }

    /// Open an existing medulla project
    pub fn open(root: &Path) -> Result<Self> {
        let medulla_dir = root.join(MEDULLA_DIR);
        let path = medulla_dir.join(LORO_DB);

        if !path.exists() {
            return Err(MedullaError::NotInitialized);
        }

        let bytes = fs::read(&path)?;
        let doc = LoroDoc::new();
        doc.import(&bytes)?;

        Ok(Self { doc, path })
    }

    /// Save the document to disk
    pub fn save(&self) -> Result<()> {
        let bytes = self.doc.export(loro::ExportMode::Snapshot)?;
        fs::write(&self.path, bytes)?;
        Ok(())
    }

    /// Get the next sequence number for a given entity type
    pub fn next_sequence_number(&self, entity_type: &str) -> u32 {
        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new()).unwrap();

        let current = sequences
            .get(entity_type)
            .and_then(|v| match v {
                ValueOrContainer::Value(LoroValue::I64(n)) => Some(n as u32),
                _ => None,
            })
            .unwrap_or(0);

        current + 1
    }

    /// Add a decision to the store
    pub fn add_decision(&self, decision: &Decision) -> Result<()> {
        let decisions = self.doc.get_map("decisions");
        let id_str = decision.base.id.to_string();

        let entity_map = decisions.get_or_create_container(&id_str, LoroMap::new())?;

        // Store base fields
        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "decision")?;
        entity_map.insert("sequence_number", decision.base.sequence_number as i64)?;
        entity_map.insert("title", decision.base.title.clone())?;
        entity_map.insert("created_at", decision.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", decision.base.updated_at.to_rfc3339())?;

        if let Some(ref content) = decision.base.content {
            entity_map.insert("content", content.clone())?;
        }

        if let Some(ref created_by) = decision.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }

        // Store decision-specific fields
        entity_map.insert("status", decision.status.to_string())?;

        if let Some(ref context) = decision.context {
            entity_map.insert("context", context.clone())?;
        }

        if let Some(ref superseded_by) = decision.superseded_by {
            entity_map.insert("superseded_by", superseded_by.clone())?;
        }

        // Store tags as LoroList
        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &decision.base.tags {
            tags_list.push(tag.clone())?;
        }

        // Store consequences as LoroList
        let consequences_list = entity_map.get_or_create_container("consequences", loro::LoroList::new())?;
        for consequence in &decision.consequences {
            consequences_list.push(consequence.clone())?;
        }

        // Update sequence counter
        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("decisions", decision.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// List all decisions
    pub fn list_decisions(&self) -> Result<Vec<Decision>> {
        let decisions_map = self.doc.get_map("decisions");
        let mut decisions = Vec::new();

        // Use the JSON export approach for simplicity
        let json = decisions_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(decision) = self.parse_decision_from_map(entity_map) {
                        decisions.push(decision);
                    }
                }
            }
        }

        // Sort by sequence number
        decisions.sort_by_key(|d| d.base.sequence_number);
        Ok(decisions)
    }

    fn parse_decision_from_map(&self, map: &loro::LoroMapValue) -> Option<Decision> {
        let id = match map.get("id")? {
            LoroValue::String(s) => s.parse().ok()?,
            _ => return None,
        };

        let title = match map.get("title")? {
            LoroValue::String(s) => s.to_string(),
            _ => return None,
        };

        let sequence_number = match map.get("sequence_number")? {
            LoroValue::I64(n) => *n as u32,
            _ => return None,
        };

        let created_at = match map.get("created_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s).ok()?.with_timezone(&chrono::Utc),
            _ => return None,
        };

        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s).ok()?.with_timezone(&chrono::Utc),
            _ => return None,
        };

        let content = map.get("content").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let created_by = map.get("created_by").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let status = map.get("status").and_then(|v| match v {
            LoroValue::String(s) => s.parse().ok(),
            _ => None,
        }).unwrap_or_default();

        let context = map.get("context").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let tags = map.get("tags").and_then(|v| match v {
            LoroValue::List(list) => Some(
                list.iter()
                    .filter_map(|item| match item {
                        LoroValue::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .collect()
            ),
            _ => None,
        }).unwrap_or_default();

        let consequences = map.get("consequences").and_then(|v| match v {
            LoroValue::List(list) => Some(
                list.iter()
                    .filter_map(|item| match item {
                        LoroValue::String(s) => Some(s.to_string()),
                        _ => None,
                    })
                    .collect()
            ),
            _ => None,
        }).unwrap_or_default();

        let superseded_by = map.get("superseded_by").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        Some(Decision {
            base: crate::entity::EntityBase {
                id,
                title,
                content,
                tags,
                created_at,
                updated_at,
                created_by,
                sequence_number,
            },
            status,
            context,
            consequences,
            superseded_by,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_medulla_directory() {
        let tmp = TempDir::new().unwrap();
        let _store = LoroStore::init(tmp.path()).unwrap();

        assert!(tmp.path().join(".medulla").exists());
        assert!(tmp.path().join(".medulla/loro.db").exists());
    }

    #[test]
    fn test_init_fails_if_already_initialized() {
        let tmp = TempDir::new().unwrap();
        LoroStore::init(tmp.path()).unwrap();

        let result = LoroStore::init(tmp.path());
        assert!(matches!(result, Err(MedullaError::AlreadyInitialized)));
    }

    #[test]
    fn test_open_fails_if_not_initialized() {
        let tmp = TempDir::new().unwrap();

        let result = LoroStore::open(tmp.path());
        assert!(matches!(result, Err(MedullaError::NotInitialized)));
    }

    #[test]
    fn test_add_and_list_decision() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut decision = Decision::new("Use Rust".to_string(), 1);
        decision.status = crate::entity::DecisionStatus::Accepted;
        decision.base.tags = vec!["architecture".to_string()];

        store.add_decision(&decision).unwrap();
        store.save().unwrap();

        // Reopen and verify
        let store2 = LoroStore::open(tmp.path()).unwrap();
        let decisions = store2.list_decisions().unwrap();

        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].base.title, "Use Rust");
        assert_eq!(decisions[0].status, crate::entity::DecisionStatus::Accepted);
    }
}
