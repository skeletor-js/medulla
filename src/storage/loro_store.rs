use std::fs;
use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroMap, LoroValue, ValueOrContainer};

use crate::cache::SqliteCache;
use crate::entity::{
    Component, Decision, DecisionStatus, Link, Note, Prompt, Relation, RelationType, Task,
    TaskPriority, TaskStatus,
};
use crate::error::{MedullaError, Result};

const MEDULLA_DIR: &str = ".medulla";
const LORO_DB: &str = "loro.db";

/// Update payload for a decision
#[derive(Default)]
pub struct DecisionUpdate {
    pub title: Option<String>,
    pub status: Option<DecisionStatus>,
    pub content: Option<String>,
    pub context: Option<String>,
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

/// Update payload for a task
#[derive(Default)]
pub struct TaskUpdate {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub content: Option<String>,
    pub due_date: Option<Option<chrono::NaiveDate>>, // Some(None) to clear, Some(Some(date)) to set
    pub assignee: Option<Option<String>>,            // Some(None) to clear, Some(Some(s)) to set
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
}

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

    /// Get the medulla directory path
    pub fn medulla_dir(&self) -> &Path {
        self.path.parent().unwrap()
    }

    /// Get a version hash for the current document state
    /// This is used for cache invalidation
    pub fn version_hash(&self) -> String {
        // Use the document's oplog vv (version vector) as a hash
        let vv = self.doc.oplog_vv();
        format!("{:?}", vv)
    }

    /// Sync the cache with the current store state
    pub fn sync_cache(&self, cache: &SqliteCache) -> Result<bool> {
        let decisions = self.list_decisions()?;
        let relations = self.list_relations()?;
        let version = self.version_hash();

        cache.sync_from_loro(&decisions, &relations, &version)
    }

    /// Get the next sequence number for a given entity type
    pub fn next_sequence_number(&self, entity_type: &str) -> u32 {
        let meta = self.doc.get_map("_meta");
        let sequences = meta
            .get_or_create_container("type_sequences", LoroMap::new())
            .unwrap();

        let current = sequences
            .get(entity_type)
            .and_then(|v| match v {
                ValueOrContainer::Value(LoroValue::I64(n)) => Some(n as u32),
                _ => None,
            })
            .unwrap_or(0);

        current + 1
    }

    /// Get a decision by UUID
    pub fn get_decision(&self, id: &uuid::Uuid) -> Result<Option<Decision>> {
        let decisions_map = self.doc.get_map("decisions");
        let id_str = id.to_string();

        let json = decisions_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_decision_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// Update an existing decision
    pub fn update_decision(&self, id: &uuid::Uuid, updates: DecisionUpdate) -> Result<()> {
        let decisions_map = self.doc.get_map("decisions");
        let id_str = id.to_string();

        // Check if the decision exists
        let entity_map = match decisions_map.get(&id_str) {
            Some(ValueOrContainer::Container(loro::Container::Map(map))) => map,
            _ => return Err(MedullaError::EntityNotFound(id_str)),
        };

        // Apply updates
        let now = chrono::Utc::now();
        entity_map.insert("updated_at", now.to_rfc3339())?;

        if let Some(title) = updates.title {
            entity_map.insert("title", title)?;
        }

        if let Some(status) = updates.status {
            entity_map.insert("status", status.to_string())?;
        }

        if let Some(content) = updates.content {
            entity_map.insert("content", content)?;
        }

        if let Some(context) = updates.context {
            entity_map.insert("context", context)?;
        }

        // Handle tag additions and removals
        if !updates.add_tags.is_empty() || !updates.remove_tags.is_empty() {
            // Get existing tags
            let existing_tags: Vec<String> = entity_map
                .get("tags")
                .and_then(|v| match v {
                    ValueOrContainer::Container(loro::Container::List(list)) => {
                        let deep = list.get_deep_value();
                        match deep {
                            LoroValue::List(items) => Some(
                                items
                                    .iter()
                                    .filter_map(|item| match item {
                                        LoroValue::String(s) => Some(s.to_string()),
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .unwrap_or_default();

            // Calculate new tags: existing + add - remove
            let mut new_tags: Vec<String> = existing_tags
                .into_iter()
                .filter(|t| !updates.remove_tags.contains(t))
                .collect();
            for tag in updates.add_tags {
                if !new_tags.contains(&tag) {
                    new_tags.push(tag);
                }
            }

            // Clear and repopulate tags list
            let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
            // Delete all existing entries
            while tags_list.len() > 0 {
                tags_list.delete(0, 1)?;
            }
            // Add new tags
            for tag in new_tags {
                tags_list.push(tag)?;
            }
        }

        self.doc.commit();
        Ok(())
    }

    /// Delete a decision by UUID
    pub fn delete_decision(&self, id: &uuid::Uuid) -> Result<()> {
        let decisions_map = self.doc.get_map("decisions");
        let id_str = id.to_string();

        // Check if it exists
        if decisions_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        decisions_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
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
        let consequences_list =
            entity_map.get_or_create_container("consequences", loro::LoroList::new())?;
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

    /// Add a relation to the store
    pub fn add_relation(&self, relation: &Relation) -> Result<()> {
        let relations_map = self.doc.get_map("relations");
        let key = relation.composite_key();

        let relation_map = relations_map.get_or_create_container(&key, LoroMap::new())?;

        relation_map.insert("source_id", relation.source_id.to_string())?;
        relation_map.insert("source_type", relation.source_type.clone())?;
        relation_map.insert("target_id", relation.target_id.to_string())?;
        relation_map.insert("target_type", relation.target_type.clone())?;
        relation_map.insert("relation_type", relation.relation_type.to_string())?;
        relation_map.insert("created_at", relation.created_at.to_rfc3339())?;

        if let Some(ref created_by) = relation.created_by {
            relation_map.insert("created_by", created_by.clone())?;
        }

        // Store properties as a nested LoroMap
        let props_map = relation_map.get_or_create_container("properties", LoroMap::new())?;
        for (k, v) in &relation.properties {
            props_map.insert(k, v.clone())?;
        }

        self.doc.commit();
        Ok(())
    }

    /// Delete a relation by its composite key
    pub fn delete_relation(&self, source_id: &str, relation_type: &str, target_id: &str) -> Result<()> {
        let relations_map = self.doc.get_map("relations");
        let key = format!("{}:{}:{}", source_id, relation_type, target_id);

        relations_map.delete(&key)?;
        self.doc.commit();
        Ok(())
    }

    /// List all relations
    pub fn list_relations(&self) -> Result<Vec<Relation>> {
        let relations_map = self.doc.get_map("relations");
        let mut relations = Vec::new();

        let json = relations_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, relation_value) in map.iter() {
                if let LoroValue::Map(relation_map) = relation_value {
                    if let Some(relation) = self.parse_relation_from_map(relation_map) {
                        relations.push(relation);
                    }
                }
            }
        }

        // Sort by created_at
        relations.sort_by_key(|r| r.created_at);
        Ok(relations)
    }

    /// Get all relations from a source entity
    pub fn get_relations_from(&self, source_id: &str) -> Result<Vec<Relation>> {
        let all_relations = self.list_relations()?;
        Ok(all_relations
            .into_iter()
            .filter(|r| r.source_id.to_string() == source_id)
            .collect())
    }

    /// Get all relations to a target entity
    pub fn get_relations_to(&self, target_id: &str) -> Result<Vec<Relation>> {
        let all_relations = self.list_relations()?;
        Ok(all_relations
            .into_iter()
            .filter(|r| r.target_id.to_string() == target_id)
            .collect())
    }

    fn parse_relation_from_map(&self, map: &loro::LoroMapValue) -> Option<Relation> {
        let source_id = match map.get("source_id")? {
            LoroValue::String(s) => s.parse().ok()?,
            _ => return None,
        };

        let source_type = match map.get("source_type")? {
            LoroValue::String(s) => s.to_string(),
            _ => return None,
        };

        let target_id = match map.get("target_id")? {
            LoroValue::String(s) => s.parse().ok()?,
            _ => return None,
        };

        let target_type = match map.get("target_type")? {
            LoroValue::String(s) => s.to_string(),
            _ => return None,
        };

        let relation_type: RelationType = match map.get("relation_type")? {
            LoroValue::String(s) => s.parse().ok()?,
            _ => return None,
        };

        let created_at = match map.get("created_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };

        let created_by = map.get("created_by").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let properties = map
            .get("properties")
            .and_then(|v| match v {
                LoroValue::Map(props) => Some(
                    props
                        .iter()
                        .filter_map(|(k, v)| match v {
                            LoroValue::String(s) => Some((k.to_string(), s.to_string())),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Relation {
            source_id,
            source_type,
            target_id,
            target_type,
            relation_type,
            created_at,
            created_by,
            properties,
        })
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
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };

        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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

        let status = map
            .get("status")
            .and_then(|v| match v {
                LoroValue::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or_default();

        let context = map.get("context").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        let consequences = map
            .get("consequences")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

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

    /// Add a task to the store
    pub fn add_task(&self, task: &Task) -> Result<()> {
        let tasks = self.doc.get_map("tasks");
        let id_str = task.base.id.to_string();

        let entity_map = tasks.get_or_create_container(&id_str, LoroMap::new())?;

        // Store base fields
        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "task")?;
        entity_map.insert("sequence_number", task.base.sequence_number as i64)?;
        entity_map.insert("title", task.base.title.clone())?;
        entity_map.insert("created_at", task.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", task.base.updated_at.to_rfc3339())?;

        if let Some(ref content) = task.base.content {
            entity_map.insert("content", content.clone())?;
        }

        if let Some(ref created_by) = task.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }

        // Store task-specific fields
        entity_map.insert("status", task.status.to_string())?;
        entity_map.insert("priority", task.priority.to_string())?;

        if let Some(ref due_date) = task.due_date {
            entity_map.insert("due_date", due_date.to_string())?;
        }

        if let Some(ref assignee) = task.assignee {
            entity_map.insert("assignee", assignee.clone())?;
        }

        // Store tags as LoroList
        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &task.base.tags {
            tags_list.push(tag.clone())?;
        }

        // Update sequence counter
        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("tasks", task.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// Get a task by UUID
    pub fn get_task(&self, id: &uuid::Uuid) -> Result<Option<Task>> {
        let tasks_map = self.doc.get_map("tasks");
        let id_str = id.to_string();

        let json = tasks_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_task_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// List all tasks
    pub fn list_tasks(&self) -> Result<Vec<Task>> {
        let tasks_map = self.doc.get_map("tasks");
        let mut tasks = Vec::new();

        let json = tasks_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(task) = self.parse_task_from_map(entity_map) {
                        tasks.push(task);
                    }
                }
            }
        }

        tasks.sort_by_key(|t| t.base.sequence_number);
        Ok(tasks)
    }

    /// Update an existing task
    pub fn update_task(&self, id: &uuid::Uuid, updates: TaskUpdate) -> Result<()> {
        let tasks_map = self.doc.get_map("tasks");
        let id_str = id.to_string();

        let entity_map = match tasks_map.get(&id_str) {
            Some(ValueOrContainer::Container(loro::Container::Map(map))) => map,
            _ => return Err(MedullaError::EntityNotFound(id_str)),
        };

        let now = chrono::Utc::now();
        entity_map.insert("updated_at", now.to_rfc3339())?;

        if let Some(title) = updates.title {
            entity_map.insert("title", title)?;
        }

        if let Some(status) = updates.status {
            entity_map.insert("status", status.to_string())?;
        }

        if let Some(priority) = updates.priority {
            entity_map.insert("priority", priority.to_string())?;
        }

        if let Some(content) = updates.content {
            entity_map.insert("content", content)?;
        }

        if let Some(due_date_opt) = updates.due_date {
            match due_date_opt {
                Some(date) => entity_map.insert("due_date", date.to_string())?,
                None => entity_map.delete("due_date")?,
            };
        }

        if let Some(assignee_opt) = updates.assignee {
            match assignee_opt {
                Some(assignee) => entity_map.insert("assignee", assignee)?,
                None => entity_map.delete("assignee")?,
            };
        }

        // Handle tag updates (same pattern as decisions)
        if !updates.add_tags.is_empty() || !updates.remove_tags.is_empty() {
            let existing_tags: Vec<String> = entity_map
                .get("tags")
                .and_then(|v| match v {
                    ValueOrContainer::Container(loro::Container::List(list)) => {
                        let deep = list.get_deep_value();
                        match deep {
                            LoroValue::List(items) => Some(
                                items
                                    .iter()
                                    .filter_map(|item| match item {
                                        LoroValue::String(s) => Some(s.to_string()),
                                        _ => None,
                                    })
                                    .collect(),
                            ),
                            _ => None,
                        }
                    }
                    _ => None,
                })
                .unwrap_or_default();

            let mut new_tags: Vec<String> = existing_tags
                .into_iter()
                .filter(|t| !updates.remove_tags.contains(t))
                .collect();
            for tag in updates.add_tags {
                if !new_tags.contains(&tag) {
                    new_tags.push(tag);
                }
            }

            let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
            while tags_list.len() > 0 {
                tags_list.delete(0, 1)?;
            }
            for tag in new_tags {
                tags_list.push(tag)?;
            }
        }

        self.doc.commit();
        Ok(())
    }

    /// Delete a task by UUID
    pub fn delete_task(&self, id: &uuid::Uuid) -> Result<()> {
        let tasks_map = self.doc.get_map("tasks");
        let id_str = id.to_string();

        if tasks_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        tasks_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
    }

    fn parse_task_from_map(&self, map: &loro::LoroMapValue) -> Option<Task> {
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
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };

        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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

        let status = map
            .get("status")
            .and_then(|v| match v {
                LoroValue::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or_default();

        let priority = map
            .get("priority")
            .and_then(|v| match v {
                LoroValue::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or_default();

        let due_date = map.get("due_date").and_then(|v| match v {
            LoroValue::String(s) => chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok(),
            _ => None,
        });

        let assignee = map.get("assignee").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });

        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Task {
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
            priority,
            due_date,
            assignee,
        })
    }

    // ========== Note Methods ==========

    /// Add a note to the store
    pub fn add_note(&self, note: &Note) -> Result<()> {
        let notes = self.doc.get_map("notes");
        let id_str = note.base.id.to_string();

        let entity_map = notes.get_or_create_container(&id_str, LoroMap::new())?;

        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "note")?;
        entity_map.insert("sequence_number", note.base.sequence_number as i64)?;
        entity_map.insert("title", note.base.title.clone())?;
        entity_map.insert("created_at", note.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", note.base.updated_at.to_rfc3339())?;

        if let Some(ref content) = note.base.content {
            entity_map.insert("content", content.clone())?;
        }
        if let Some(ref created_by) = note.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }
        if let Some(ref note_type) = note.note_type {
            entity_map.insert("note_type", note_type.clone())?;
        }

        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &note.base.tags {
            tags_list.push(tag.clone())?;
        }

        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("notes", note.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// Get a note by UUID
    pub fn get_note(&self, id: &uuid::Uuid) -> Result<Option<Note>> {
        let notes_map = self.doc.get_map("notes");
        let id_str = id.to_string();

        let json = notes_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_note_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// List all notes
    pub fn list_notes(&self) -> Result<Vec<Note>> {
        let notes_map = self.doc.get_map("notes");
        let mut notes = Vec::new();

        let json = notes_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(note) = self.parse_note_from_map(entity_map) {
                        notes.push(note);
                    }
                }
            }
        }

        notes.sort_by_key(|n| n.base.sequence_number);
        Ok(notes)
    }

    /// Delete a note by UUID
    pub fn delete_note(&self, id: &uuid::Uuid) -> Result<()> {
        let notes_map = self.doc.get_map("notes");
        let id_str = id.to_string();

        if notes_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        notes_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
    }

    fn parse_note_from_map(&self, map: &loro::LoroMapValue) -> Option<Note> {
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
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };
        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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
        let note_type = map.get("note_type").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Note {
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
            note_type,
        })
    }

    // ========== Prompt Methods ==========

    /// Add a prompt to the store
    pub fn add_prompt(&self, prompt: &Prompt) -> Result<()> {
        let prompts = self.doc.get_map("prompts");
        let id_str = prompt.base.id.to_string();

        let entity_map = prompts.get_or_create_container(&id_str, LoroMap::new())?;

        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "prompt")?;
        entity_map.insert("sequence_number", prompt.base.sequence_number as i64)?;
        entity_map.insert("title", prompt.base.title.clone())?;
        entity_map.insert("created_at", prompt.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", prompt.base.updated_at.to_rfc3339())?;

        if let Some(ref content) = prompt.base.content {
            entity_map.insert("content", content.clone())?;
        }
        if let Some(ref created_by) = prompt.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }
        if let Some(ref template) = prompt.template {
            entity_map.insert("template", template.clone())?;
        }
        if let Some(ref output_schema) = prompt.output_schema {
            entity_map.insert("output_schema", output_schema.clone())?;
        }

        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &prompt.base.tags {
            tags_list.push(tag.clone())?;
        }

        let variables_list = entity_map.get_or_create_container("variables", loro::LoroList::new())?;
        for var in &prompt.variables {
            variables_list.push(var.clone())?;
        }

        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("prompts", prompt.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// Get a prompt by UUID
    pub fn get_prompt(&self, id: &uuid::Uuid) -> Result<Option<Prompt>> {
        let prompts_map = self.doc.get_map("prompts");
        let id_str = id.to_string();

        let json = prompts_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_prompt_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// List all prompts
    pub fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let prompts_map = self.doc.get_map("prompts");
        let mut prompts = Vec::new();

        let json = prompts_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(prompt) = self.parse_prompt_from_map(entity_map) {
                        prompts.push(prompt);
                    }
                }
            }
        }

        prompts.sort_by_key(|p| p.base.sequence_number);
        Ok(prompts)
    }

    /// Delete a prompt by UUID
    pub fn delete_prompt(&self, id: &uuid::Uuid) -> Result<()> {
        let prompts_map = self.doc.get_map("prompts");
        let id_str = id.to_string();

        if prompts_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        prompts_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
    }

    fn parse_prompt_from_map(&self, map: &loro::LoroMapValue) -> Option<Prompt> {
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
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };
        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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
        let template = map.get("template").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let output_schema = map.get("output_schema").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();
        let variables = map
            .get("variables")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Prompt {
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
            template,
            variables,
            output_schema,
        })
    }

    // ========== Component Methods ==========

    /// Add a component to the store
    pub fn add_component(&self, component: &Component) -> Result<()> {
        let components = self.doc.get_map("components");
        let id_str = component.base.id.to_string();

        let entity_map = components.get_or_create_container(&id_str, LoroMap::new())?;

        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "component")?;
        entity_map.insert("sequence_number", component.base.sequence_number as i64)?;
        entity_map.insert("title", component.base.title.clone())?;
        entity_map.insert("created_at", component.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", component.base.updated_at.to_rfc3339())?;
        entity_map.insert("status", component.status.to_string())?;

        if let Some(ref content) = component.base.content {
            entity_map.insert("content", content.clone())?;
        }
        if let Some(ref created_by) = component.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }
        if let Some(ref component_type) = component.component_type {
            entity_map.insert("component_type", component_type.clone())?;
        }
        if let Some(ref owner) = component.owner {
            entity_map.insert("owner", owner.clone())?;
        }

        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &component.base.tags {
            tags_list.push(tag.clone())?;
        }

        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("components", component.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// Get a component by UUID
    pub fn get_component(&self, id: &uuid::Uuid) -> Result<Option<Component>> {
        let components_map = self.doc.get_map("components");
        let id_str = id.to_string();

        let json = components_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_component_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// List all components
    pub fn list_components(&self) -> Result<Vec<Component>> {
        let components_map = self.doc.get_map("components");
        let mut components = Vec::new();

        let json = components_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(component) = self.parse_component_from_map(entity_map) {
                        components.push(component);
                    }
                }
            }
        }

        components.sort_by_key(|c| c.base.sequence_number);
        Ok(components)
    }

    /// Delete a component by UUID
    pub fn delete_component(&self, id: &uuid::Uuid) -> Result<()> {
        let components_map = self.doc.get_map("components");
        let id_str = id.to_string();

        if components_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        components_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
    }

    fn parse_component_from_map(&self, map: &loro::LoroMapValue) -> Option<Component> {
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
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };
        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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
        let status = map
            .get("status")
            .and_then(|v| match v {
                LoroValue::String(s) => s.parse().ok(),
                _ => None,
            })
            .unwrap_or_default();
        let component_type = map.get("component_type").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let owner = map.get("owner").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Component {
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
            component_type,
            status,
            owner,
        })
    }

    // ========== Link Methods ==========

    /// Add a link to the store
    pub fn add_link(&self, link: &Link) -> Result<()> {
        let links = self.doc.get_map("links");
        let id_str = link.base.id.to_string();

        let entity_map = links.get_or_create_container(&id_str, LoroMap::new())?;

        entity_map.insert("id", id_str.clone())?;
        entity_map.insert("type", "link")?;
        entity_map.insert("sequence_number", link.base.sequence_number as i64)?;
        entity_map.insert("title", link.base.title.clone())?;
        entity_map.insert("url", link.url.clone())?;
        entity_map.insert("created_at", link.base.created_at.to_rfc3339())?;
        entity_map.insert("updated_at", link.base.updated_at.to_rfc3339())?;

        if let Some(ref content) = link.base.content {
            entity_map.insert("content", content.clone())?;
        }
        if let Some(ref created_by) = link.base.created_by {
            entity_map.insert("created_by", created_by.clone())?;
        }
        if let Some(ref link_type) = link.link_type {
            entity_map.insert("link_type", link_type.clone())?;
        }

        let tags_list = entity_map.get_or_create_container("tags", loro::LoroList::new())?;
        for tag in &link.base.tags {
            tags_list.push(tag.clone())?;
        }

        let meta = self.doc.get_map("_meta");
        let sequences = meta.get_or_create_container("type_sequences", LoroMap::new())?;
        sequences.insert("links", link.base.sequence_number as i64)?;

        self.doc.commit();
        Ok(())
    }

    /// Get a link by UUID
    pub fn get_link(&self, id: &uuid::Uuid) -> Result<Option<Link>> {
        let links_map = self.doc.get_map("links");
        let id_str = id.to_string();

        let json = links_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            if let Some(LoroValue::Map(entity_map)) = map.get(&id_str) {
                return Ok(self.parse_link_from_map(entity_map));
            }
        }
        Ok(None)
    }

    /// List all links
    pub fn list_links(&self) -> Result<Vec<Link>> {
        let links_map = self.doc.get_map("links");
        let mut links = Vec::new();

        let json = links_map.get_deep_value();
        if let LoroValue::Map(map) = json {
            for (_, entity_value) in map.iter() {
                if let LoroValue::Map(entity_map) = entity_value {
                    if let Some(link) = self.parse_link_from_map(entity_map) {
                        links.push(link);
                    }
                }
            }
        }

        links.sort_by_key(|l| l.base.sequence_number);
        Ok(links)
    }

    /// Delete a link by UUID
    pub fn delete_link(&self, id: &uuid::Uuid) -> Result<()> {
        let links_map = self.doc.get_map("links");
        let id_str = id.to_string();

        if links_map.get(&id_str).is_none() {
            return Err(MedullaError::EntityNotFound(id_str));
        }

        links_map.delete(&id_str)?;
        self.doc.commit();
        Ok(())
    }

    fn parse_link_from_map(&self, map: &loro::LoroMapValue) -> Option<Link> {
        let id = match map.get("id")? {
            LoroValue::String(s) => s.parse().ok()?,
            _ => return None,
        };
        let title = match map.get("title")? {
            LoroValue::String(s) => s.to_string(),
            _ => return None,
        };
        let url = match map.get("url")? {
            LoroValue::String(s) => s.to_string(),
            _ => return None,
        };
        let sequence_number = match map.get("sequence_number")? {
            LoroValue::I64(n) => *n as u32,
            _ => return None,
        };
        let created_at = match map.get("created_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
            _ => return None,
        };
        let updated_at = match map.get("updated_at")? {
            LoroValue::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                .ok()?
                .with_timezone(&chrono::Utc),
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
        let link_type = map.get("link_type").and_then(|v| match v {
            LoroValue::String(s) => Some(s.to_string()),
            _ => None,
        });
        let tags = map
            .get("tags")
            .and_then(|v| match v {
                LoroValue::List(list) => Some(
                    list.iter()
                        .filter_map(|item| match item {
                            LoroValue::String(s) => Some(s.to_string()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default();

        Some(Link {
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
            url,
            link_type,
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

    #[test]
    fn test_add_and_list_relation() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        // Create two decisions
        let decision1 = Decision::new("Use Rust".to_string(), 1);
        let decision2 = Decision::new("Use Loro".to_string(), 2);

        store.add_decision(&decision1).unwrap();
        store.add_decision(&decision2).unwrap();

        // Create a relation: decision2 implements decision1
        let relation = crate::entity::Relation::new(
            decision2.base.id,
            "decision".to_string(),
            decision1.base.id,
            "decision".to_string(),
            crate::entity::RelationType::Implements,
        );

        store.add_relation(&relation).unwrap();
        store.save().unwrap();

        // Reopen and verify
        let store2 = LoroStore::open(tmp.path()).unwrap();
        let relations = store2.list_relations().unwrap();

        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source_id, decision2.base.id);
        assert_eq!(relations[0].target_id, decision1.base.id);
        assert_eq!(relations[0].relation_type, crate::entity::RelationType::Implements);
    }

    #[test]
    fn test_get_relations_from_and_to() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let decision1 = Decision::new("Decision A".to_string(), 1);
        let decision2 = Decision::new("Decision B".to_string(), 2);
        let decision3 = Decision::new("Decision C".to_string(), 3);

        store.add_decision(&decision1).unwrap();
        store.add_decision(&decision2).unwrap();
        store.add_decision(&decision3).unwrap();

        // decision2 supersedes decision1
        let rel1 = crate::entity::Relation::new(
            decision2.base.id,
            "decision".to_string(),
            decision1.base.id,
            "decision".to_string(),
            crate::entity::RelationType::Supersedes,
        );

        // decision3 references decision2
        let rel2 = crate::entity::Relation::new(
            decision3.base.id,
            "decision".to_string(),
            decision2.base.id,
            "decision".to_string(),
            crate::entity::RelationType::References,
        );

        store.add_relation(&rel1).unwrap();
        store.add_relation(&rel2).unwrap();
        store.save().unwrap();

        // Test get_relations_from
        let from_d2 = store.get_relations_from(&decision2.base.id.to_string()).unwrap();
        assert_eq!(from_d2.len(), 1);
        assert_eq!(from_d2[0].relation_type, crate::entity::RelationType::Supersedes);

        // Test get_relations_to
        let to_d2 = store.get_relations_to(&decision2.base.id.to_string()).unwrap();
        assert_eq!(to_d2.len(), 1);
        assert_eq!(to_d2[0].relation_type, crate::entity::RelationType::References);
    }

    #[test]
    fn test_delete_relation() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let decision1 = Decision::new("Decision A".to_string(), 1);
        let decision2 = Decision::new("Decision B".to_string(), 2);

        store.add_decision(&decision1).unwrap();
        store.add_decision(&decision2).unwrap();

        let relation = crate::entity::Relation::new(
            decision2.base.id,
            "decision".to_string(),
            decision1.base.id,
            "decision".to_string(),
            crate::entity::RelationType::Supersedes,
        );

        store.add_relation(&relation).unwrap();
        assert_eq!(store.list_relations().unwrap().len(), 1);

        // Delete the relation
        store.delete_relation(
            &decision2.base.id.to_string(),
            "supersedes",
            &decision1.base.id.to_string(),
        ).unwrap();
        store.save().unwrap();

        assert_eq!(store.list_relations().unwrap().len(), 0);
    }

    #[test]
    fn test_get_decision() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let decision = Decision::new("Test Decision".to_string(), 1);
        let decision_id = decision.base.id;

        store.add_decision(&decision).unwrap();
        store.save().unwrap();

        // Get existing decision
        let retrieved = store.get_decision(&decision_id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().base.title, "Test Decision");

        // Get non-existing decision
        let non_existing = store.get_decision(&uuid::Uuid::new_v4()).unwrap();
        assert!(non_existing.is_none());
    }

    #[test]
    fn test_update_decision() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut decision = Decision::new("Original Title".to_string(), 1);
        decision.base.tags = vec!["tag1".to_string(), "tag2".to_string()];
        decision.status = crate::entity::DecisionStatus::Proposed;

        store.add_decision(&decision).unwrap();

        // Update title and status
        let mut updates = DecisionUpdate::default();
        updates.title = Some("Updated Title".to_string());
        updates.status = Some(crate::entity::DecisionStatus::Accepted);
        updates.add_tags = vec!["tag3".to_string()];
        updates.remove_tags = vec!["tag1".to_string()];

        store.update_decision(&decision.base.id, updates).unwrap();
        store.save().unwrap();

        // Reopen and verify
        let store2 = LoroStore::open(tmp.path()).unwrap();
        let updated = store2.get_decision(&decision.base.id).unwrap().unwrap();

        assert_eq!(updated.base.title, "Updated Title");
        assert_eq!(updated.status, crate::entity::DecisionStatus::Accepted);
        assert_eq!(updated.base.tags, vec!["tag2".to_string(), "tag3".to_string()]);
    }

    #[test]
    fn test_delete_decision() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let decision = Decision::new("To Be Deleted".to_string(), 1);
        let decision_id = decision.base.id;

        store.add_decision(&decision).unwrap();
        assert_eq!(store.list_decisions().unwrap().len(), 1);

        // Delete the decision
        store.delete_decision(&decision_id).unwrap();
        store.save().unwrap();

        assert_eq!(store.list_decisions().unwrap().len(), 0);

        // Verify it's really gone
        let store2 = LoroStore::open(tmp.path()).unwrap();
        assert!(store2.get_decision(&decision_id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_decision_fails() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let result = store.delete_decision(&uuid::Uuid::new_v4());
        assert!(matches!(result, Err(MedullaError::EntityNotFound(_))));
    }

    #[test]
    fn test_add_and_list_task() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut task = Task::new("Implement feature".to_string(), 1);
        task.status = TaskStatus::InProgress;
        task.priority = TaskPriority::High;
        task.due_date = Some(chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());

        store.add_task(&task).unwrap();
        store.save().unwrap();

        let store2 = LoroStore::open(tmp.path()).unwrap();
        let tasks = store2.list_tasks().unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].base.title, "Implement feature");
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(tasks[0].priority, TaskPriority::High);
    }

    #[test]
    fn test_add_and_list_note() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut note = Note::new("Meeting notes".to_string(), 1);
        note.note_type = Some("meeting".to_string());
        note.base.content = Some("Discussed project timeline".to_string());

        store.add_note(&note).unwrap();
        store.save().unwrap();

        let store2 = LoroStore::open(tmp.path()).unwrap();
        let notes = store2.list_notes().unwrap();

        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].base.title, "Meeting notes");
        assert_eq!(notes[0].note_type, Some("meeting".to_string()));
    }

    #[test]
    fn test_add_and_list_prompt() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut prompt = Prompt::new("Code review".to_string(), 1);
        prompt.template = Some("Review this code: {{code}}".to_string());
        prompt.variables = vec!["code".to_string()];

        store.add_prompt(&prompt).unwrap();
        store.save().unwrap();

        let store2 = LoroStore::open(tmp.path()).unwrap();
        let prompts = store2.list_prompts().unwrap();

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].base.title, "Code review");
        assert_eq!(prompts[0].variables, vec!["code".to_string()]);
    }

    #[test]
    fn test_add_and_list_component() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut component = Component::new("Auth Service".to_string(), 1);
        component.component_type = Some("service".to_string());
        component.status = crate::entity::ComponentStatus::Active;
        component.owner = Some("team-auth".to_string());

        store.add_component(&component).unwrap();
        store.save().unwrap();

        let store2 = LoroStore::open(tmp.path()).unwrap();
        let components = store2.list_components().unwrap();

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].base.title, "Auth Service");
        assert_eq!(components[0].status, crate::entity::ComponentStatus::Active);
    }

    #[test]
    fn test_add_and_list_link() {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();

        let mut link = Link::new(
            "Project Docs".to_string(),
            "https://docs.example.com".to_string(),
            1,
        );
        link.link_type = Some("documentation".to_string());

        store.add_link(&link).unwrap();
        store.save().unwrap();

        let store2 = LoroStore::open(tmp.path()).unwrap();
        let links = store2.list_links().unwrap();

        assert_eq!(links.len(), 1);
        assert_eq!(links[0].base.title, "Project Docs");
        assert_eq!(links[0].url, "https://docs.example.com");
    }
}
