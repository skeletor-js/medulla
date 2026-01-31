# Remaining Entity Types Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the remaining five entity types (task, note, prompt, component, link) to complete Phase 1.

**Architecture:** Each entity type follows the established pattern: a struct with `EntityBase` + type-specific fields, stored in Loro under its own top-level map, with CLI commands for CRUD operations and SQLite cache indexing.

**Tech Stack:** Rust, Loro CRDT, SQLite FTS5, clap CLI

---

## Task 1: Implement Task Entity Type

**Files:**
- Create: `src/entity/task.rs`
- Modify: `src/entity/mod.rs:1-10`

**Step 1: Create task.rs with TaskStatus enum and Task struct**

```rust
// src/entity/task.rs
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::EntityBase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Done,
    Blocked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Todo => write!(f, "todo"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Done => write!(f, "done"),
            TaskStatus::Blocked => write!(f, "blocked"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "todo" => Ok(TaskStatus::Todo),
            "in_progress" | "inprogress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            "blocked" => Ok(TaskStatus::Blocked),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Normal => write!(f, "normal"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(TaskPriority::Low),
            "normal" => Ok(TaskPriority::Normal),
            "high" => Ok(TaskPriority::High),
            "urgent" => Ok(TaskPriority::Urgent),
            _ => Err(format!("Invalid task priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    #[serde(flatten)]
    pub base: EntityBase,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub due_date: Option<NaiveDate>,
    pub assignee: Option<String>,
}

impl Task {
    pub fn new(title: String, sequence_number: u32) -> Self {
        Self {
            base: EntityBase::new(title, sequence_number),
            status: TaskStatus::default(),
            priority: TaskPriority::default(),
            due_date: None,
            assignee: None,
        }
    }
}
```

**Step 2: Update src/entity/mod.rs to export Task**

```rust
// src/entity/mod.rs
mod decision;
mod relation;
mod task;

pub use decision::{Decision, DecisionStatus};
pub use relation::{Relation, RelationType};
pub use task::{Task, TaskPriority, TaskStatus};

// ... rest unchanged
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/entity/task.rs src/entity/mod.rs
git commit -m "feat(entity): add Task entity type with status and priority"
```

---

## Task 2: Implement Note Entity Type

**Files:**
- Create: `src/entity/note.rs`
- Modify: `src/entity/mod.rs`

**Step 1: Create note.rs**

```rust
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
```

**Step 2: Update src/entity/mod.rs**

```rust
mod decision;
mod note;
mod relation;
mod task;

pub use decision::{Decision, DecisionStatus};
pub use note::Note;
pub use relation::{Relation, RelationType};
pub use task::{Task, TaskPriority, TaskStatus};
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/entity/note.rs src/entity/mod.rs
git commit -m "feat(entity): add Note entity type"
```

---

## Task 3: Implement Prompt Entity Type

**Files:**
- Create: `src/entity/prompt.rs`
- Modify: `src/entity/mod.rs`

**Step 1: Create prompt.rs**

```rust
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
```

**Step 2: Update src/entity/mod.rs**

```rust
mod decision;
mod note;
mod prompt;
mod relation;
mod task;

pub use decision::{Decision, DecisionStatus};
pub use note::Note;
pub use prompt::Prompt;
pub use relation::{Relation, RelationType};
pub use task::{Task, TaskPriority, TaskStatus};
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/entity/prompt.rs src/entity/mod.rs
git commit -m "feat(entity): add Prompt entity type with template and variables"
```

---

## Task 4: Implement Component Entity Type

**Files:**
- Create: `src/entity/component.rs`
- Modify: `src/entity/mod.rs`

**Step 1: Create component.rs**

```rust
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
```

**Step 2: Update src/entity/mod.rs**

```rust
mod component;
mod decision;
mod note;
mod prompt;
mod relation;
mod task;

pub use component::{Component, ComponentStatus};
pub use decision::{Decision, DecisionStatus};
pub use note::Note;
pub use prompt::Prompt;
pub use relation::{Relation, RelationType};
pub use task::{Task, TaskPriority, TaskStatus};
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/entity/component.rs src/entity/mod.rs
git commit -m "feat(entity): add Component entity type with status and owner"
```

---

## Task 5: Implement Link Entity Type

**Files:**
- Create: `src/entity/link.rs`
- Modify: `src/entity/mod.rs`

**Step 1: Create link.rs**

```rust
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
```

**Step 2: Update src/entity/mod.rs**

```rust
mod component;
mod decision;
mod link;
mod note;
mod prompt;
mod relation;
mod task;

pub use component::{Component, ComponentStatus};
pub use decision::{Decision, DecisionStatus};
pub use link::Link;
pub use note::Note;
pub use prompt::Prompt;
pub use relation::{Relation, RelationType};
pub use task::{Task, TaskPriority, TaskStatus};
```

**Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 4: Commit**

```bash
git add src/entity/link.rs src/entity/mod.rs
git commit -m "feat(entity): add Link entity type with URL"
```

---

## Task 6: Add Loro Storage Methods for Task

**Files:**
- Modify: `src/storage/loro_store.rs`

**Step 1: Add TaskUpdate struct and task storage methods**

Add after `DecisionUpdate` struct (around line 22):

```rust
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
```

Add imports at top of file:

```rust
use crate::entity::{Decision, DecisionStatus, Relation, RelationType, Task, TaskPriority, TaskStatus};
```

Add these methods to `impl LoroStore` block:

```rust
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
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src/storage/loro_store.rs
git commit -m "feat(storage): add Loro storage methods for Task entity"
```

---

## Task 7: Add Loro Storage Methods for Note, Prompt, Component, Link

**Files:**
- Modify: `src/storage/loro_store.rs`

This task adds storage methods for the remaining four entity types. Follow the same pattern as Task - add/get/list/update/delete methods plus a parse helper.

**Step 1: Add imports for remaining entity types**

Update import line:

```rust
use crate::entity::{
    Component, ComponentStatus, Decision, DecisionStatus, Link, Note, Prompt,
    Relation, RelationType, Task, TaskPriority, TaskStatus,
};
```

**Step 2: Add Note storage methods**

```rust
// Note methods
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
```

**Step 3: Add Prompt storage methods**

```rust
// Prompt methods
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
```

**Step 4: Add Component storage methods**

```rust
// Component methods
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
```

**Step 5: Add Link storage methods**

```rust
// Link methods
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
```

**Step 6: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 7: Commit**

```bash
git add src/storage/loro_store.rs
git commit -m "feat(storage): add Loro storage methods for Note, Prompt, Component, Link"
```

---

## Task 8: Add CLI Commands for All Entity Types

**Files:**
- Modify: `src/cli/commands.rs`

**Step 1: Expand AddEntity enum with all types**

Replace the `AddEntity` enum:

```rust
#[derive(Subcommand, Debug)]
pub enum AddEntity {
    /// Add a new decision
    Decision {
        /// Decision title
        title: String,

        /// Decision status (proposed, accepted, deprecated, superseded)
        #[arg(long, default_value = "proposed")]
        status: String,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id" (can be specified multiple times)
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Open $EDITOR for content
        #[arg(long, short = 'e')]
        edit: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new task
    Task {
        /// Task title
        title: String,

        /// Task status (todo, in_progress, done, blocked)
        #[arg(long, default_value = "todo")]
        status: String,

        /// Priority (low, normal, high, urgent)
        #[arg(long, default_value = "normal")]
        priority: String,

        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,

        /// Assignee
        #[arg(long)]
        assignee: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new note
    Note {
        /// Note title
        title: String,

        /// Note type (e.g., "meeting", "research", "idea")
        #[arg(long = "type")]
        note_type: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new prompt template
    Prompt {
        /// Prompt title
        title: String,

        /// Template text (use {{var}} for variables)
        #[arg(long)]
        template: Option<String>,

        /// Variables (can be specified multiple times)
        #[arg(long = "var")]
        variables: Vec<String>,

        /// Output JSON schema
        #[arg(long)]
        output_schema: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Read template from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new component
    Component {
        /// Component title/name
        title: String,

        /// Component type (e.g., "service", "library", "api")
        #[arg(long = "type")]
        component_type: Option<String>,

        /// Status (active, deprecated, planned)
        #[arg(long, default_value = "active")]
        status: String,

        /// Owner
        #[arg(long)]
        owner: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Read content from stdin
        #[arg(long)]
        stdin: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Add a new link
    Link {
        /// Link title/description
        title: String,

        /// URL (required)
        #[arg(long)]
        url: String,

        /// Link type (e.g., "documentation", "issue", "pr")
        #[arg(long = "type")]
        link_type: Option<String>,

        /// Tags (can be specified multiple times)
        #[arg(long = "tag", short = 't')]
        tags: Vec<String>,

        /// Relations in format "type:target_id"
        #[arg(long = "relation", short = 'r')]
        relations: Vec<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
```

**Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles (handlers not yet implemented, but structure is valid)

**Step 3: Commit**

```bash
git add src/cli/commands.rs
git commit -m "feat(cli): add CLI commands for task, note, prompt, component, link"
```

---

## Task 9: Add CLI Handlers for All Entity Types

**Files:**
- Modify: `src/cli/handlers.rs`
- Modify: `src/main.rs`

**Step 1: Add imports and handler functions in handlers.rs**

Add to imports:

```rust
use crate::entity::{
    Component, ComponentStatus, Decision, DecisionStatus, Link, Note, Prompt,
    Relation, RelationType, Task, TaskPriority, TaskStatus,
};
```

Add handler functions (after `handle_add_decision`):

```rust
pub fn handle_add_task(
    title: String,
    status: String,
    priority: String,
    due: Option<String>,
    assignee: Option<String>,
    tags: Vec<String>,
    relations: Vec<String>,
    stdin: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("tasks");
    let mut task = Task::new(title, seq);

    task.status = status.parse().unwrap_or_default();
    task.priority = priority.parse().unwrap_or_default();
    task.due_date = due.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
    task.assignee = assignee;
    task.base.tags = tags;

    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            task.base.content = Some(content);
        }
    }

    let git_author = get_git_author();
    task.base.created_by = git_author.clone();

    store.add_task(&task)?;
    add_relations_for_entity(&store, task.base.id, "task", &relations, &git_author)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&task)?);
    } else {
        println!(
            "Created task {:03} ({}) - {}",
            task.base.sequence_number,
            &task.base.id.to_string()[..7],
            task.base.title
        );
    }

    Ok(())
}

pub fn handle_add_note(
    title: String,
    note_type: Option<String>,
    tags: Vec<String>,
    relations: Vec<String>,
    stdin: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("notes");
    let mut note = Note::new(title, seq);

    note.note_type = note_type;
    note.base.tags = tags;

    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            note.base.content = Some(content);
        }
    }

    let git_author = get_git_author();
    note.base.created_by = git_author.clone();

    store.add_note(&note)?;
    add_relations_for_entity(&store, note.base.id, "note", &relations, &git_author)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&note)?);
    } else {
        println!(
            "Created note {:03} ({}) - {}",
            note.base.sequence_number,
            &note.base.id.to_string()[..7],
            note.base.title
        );
    }

    Ok(())
}

pub fn handle_add_prompt(
    title: String,
    template: Option<String>,
    variables: Vec<String>,
    output_schema: Option<String>,
    tags: Vec<String>,
    stdin: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("prompts");
    let mut prompt = Prompt::new(title, seq);

    prompt.variables = variables;
    prompt.output_schema = output_schema;
    prompt.base.tags = tags;

    // Template can come from --template flag or stdin
    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            prompt.template = Some(content);
        }
    } else {
        prompt.template = template;
    }

    let git_author = get_git_author();
    prompt.base.created_by = git_author;

    store.add_prompt(&prompt)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&prompt)?);
    } else {
        println!(
            "Created prompt {:03} ({}) - {}",
            prompt.base.sequence_number,
            &prompt.base.id.to_string()[..7],
            prompt.base.title
        );
    }

    Ok(())
}

pub fn handle_add_component(
    title: String,
    component_type: Option<String>,
    status: String,
    owner: Option<String>,
    tags: Vec<String>,
    relations: Vec<String>,
    stdin: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("components");
    let mut component = Component::new(title, seq);

    component.component_type = component_type;
    component.status = status.parse().unwrap_or_default();
    component.owner = owner;
    component.base.tags = tags;

    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            component.base.content = Some(content);
        }
    }

    let git_author = get_git_author();
    component.base.created_by = git_author.clone();

    store.add_component(&component)?;
    add_relations_for_entity(&store, component.base.id, "component", &relations, &git_author)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&component)?);
    } else {
        println!(
            "Created component {:03} ({}) - {}",
            component.base.sequence_number,
            &component.base.id.to_string()[..7],
            component.base.title
        );
    }

    Ok(())
}

pub fn handle_add_link(
    title: String,
    url: String,
    link_type: Option<String>,
    tags: Vec<String>,
    relations: Vec<String>,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("links");
    let mut link = Link::new(title, url, seq);

    link.link_type = link_type;
    link.base.tags = tags;

    let git_author = get_git_author();
    link.base.created_by = git_author.clone();

    store.add_link(&link)?;
    add_relations_for_entity(&store, link.base.id, "link", &relations, &git_author)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&link)?);
    } else {
        println!(
            "Created link {:03} ({}) - {}",
            link.base.sequence_number,
            &link.base.id.to_string()[..7],
            link.base.title
        );
    }

    Ok(())
}

/// Helper to add relations for any entity type
fn add_relations_for_entity(
    store: &LoroStore,
    source_id: uuid::Uuid,
    source_type: &str,
    relations: &[String],
    git_author: &Option<String>,
) -> Result<()> {
    for rel_str in relations {
        match parse_relation_string(rel_str) {
            Ok((rel_type, target_id)) => {
                let mut relation = Relation::new(
                    source_id,
                    source_type.to_string(),
                    target_id,
                    "unknown".to_string(),
                    rel_type,
                );
                relation.created_by = git_author.clone();
                if let Err(e) = store.add_relation(&relation) {
                    eprintln!("Warning: failed to add relation '{}': {}", rel_str, e);
                }
            }
            Err(e) => {
                eprintln!("Warning: invalid relation '{}': {}", rel_str, e);
            }
        }
    }
    Ok(())
}
```

**Step 2: Update handle_list to support all types**

Replace `handle_list`:

```rust
pub fn handle_list(entity_type: Option<String>, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let entity_type = entity_type.as_deref().unwrap_or("decision");

    match entity_type {
        "decision" | "decisions" => {
            let decisions = store.list_decisions()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&decisions)?);
            } else if decisions.is_empty() {
                println!("No decisions found.");
            } else {
                println!("Decisions:\n");
                for d in decisions {
                    println!(
                        "  {:03} ({}) [{}] {}",
                        d.base.sequence_number,
                        &d.base.id.to_string()[..7],
                        d.status,
                        d.base.title
                    );
                    if !d.base.tags.is_empty() {
                        println!("      tags: {}", d.base.tags.join(", "));
                    }
                }
            }
        }
        "task" | "tasks" => {
            let tasks = store.list_tasks()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else if tasks.is_empty() {
                println!("No tasks found.");
            } else {
                println!("Tasks:\n");
                for t in tasks {
                    let due_str = t.due_date.map(|d| format!(" due:{}", d)).unwrap_or_default();
                    println!(
                        "  {:03} ({}) [{}|{}]{} {}",
                        t.base.sequence_number,
                        &t.base.id.to_string()[..7],
                        t.status,
                        t.priority,
                        due_str,
                        t.base.title
                    );
                }
            }
        }
        "note" | "notes" => {
            let notes = store.list_notes()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&notes)?);
            } else if notes.is_empty() {
                println!("No notes found.");
            } else {
                println!("Notes:\n");
                for n in notes {
                    let type_str = n.note_type.as_deref().unwrap_or("note");
                    println!(
                        "  {:03} ({}) [{}] {}",
                        n.base.sequence_number,
                        &n.base.id.to_string()[..7],
                        type_str,
                        n.base.title
                    );
                }
            }
        }
        "prompt" | "prompts" => {
            let prompts = store.list_prompts()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&prompts)?);
            } else if prompts.is_empty() {
                println!("No prompts found.");
            } else {
                println!("Prompts:\n");
                for p in prompts {
                    let vars = if p.variables.is_empty() {
                        String::new()
                    } else {
                        format!(" vars: {}", p.variables.join(", "))
                    };
                    println!(
                        "  {:03} ({}) {}{}",
                        p.base.sequence_number,
                        &p.base.id.to_string()[..7],
                        p.base.title,
                        vars
                    );
                }
            }
        }
        "component" | "components" => {
            let components = store.list_components()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&components)?);
            } else if components.is_empty() {
                println!("No components found.");
            } else {
                println!("Components:\n");
                for c in components {
                    let type_str = c.component_type.as_deref().unwrap_or("component");
                    println!(
                        "  {:03} ({}) [{}|{}] {}",
                        c.base.sequence_number,
                        &c.base.id.to_string()[..7],
                        type_str,
                        c.status,
                        c.base.title
                    );
                }
            }
        }
        "link" | "links" => {
            let links = store.list_links()?;
            if json {
                println!("{}", serde_json::to_string_pretty(&links)?);
            } else if links.is_empty() {
                println!("No links found.");
            } else {
                println!("Links:\n");
                for l in links {
                    println!(
                        "  {:03} ({}) {} -> {}",
                        l.base.sequence_number,
                        &l.base.id.to_string()[..7],
                        l.base.title,
                        l.url
                    );
                }
            }
        }
        _ => {
            eprintln!(
                "Unknown entity type '{}'. Valid types: decision, task, note, prompt, component, link",
                entity_type
            );
        }
    }

    Ok(())
}
```

**Step 3: Update main.rs to wire up new handlers**

In `main.rs`, update the `AddEntity` match arm:

```rust
Commands::Add(add_cmd) => match add_cmd.entity {
    AddEntity::Decision { title, status, tags, relations, stdin, edit, json } => {
        handlers::handle_add_decision(title, status, tags, relations, stdin, edit, json)
    }
    AddEntity::Task { title, status, priority, due, assignee, tags, relations, stdin, json } => {
        handlers::handle_add_task(title, status, priority, due, assignee, tags, relations, stdin, json)
    }
    AddEntity::Note { title, note_type, tags, relations, stdin, json } => {
        handlers::handle_add_note(title, note_type, tags, relations, stdin, json)
    }
    AddEntity::Prompt { title, template, variables, output_schema, tags, stdin, json } => {
        handlers::handle_add_prompt(title, template, variables, output_schema, tags, stdin, json)
    }
    AddEntity::Component { title, component_type, status, owner, tags, relations, stdin, json } => {
        handlers::handle_add_component(title, component_type, status, owner, tags, relations, stdin, json)
    }
    AddEntity::Link { title, url, link_type, tags, relations, json } => {
        handlers::handle_add_link(title, url, link_type, tags, relations, json)
    }
},
```

**Step 4: Run cargo check**

Run: `cargo check`
Expected: Compiles without errors

**Step 5: Commit**

```bash
git add src/cli/handlers.rs src/main.rs
git commit -m "feat(cli): implement handlers for all entity types"
```

---

## Task 10: Add Unit Tests for New Entity Types

**Files:**
- Modify: `src/storage/loro_store.rs` (add tests at bottom)

**Step 1: Add tests for Task storage**

Add to the `#[cfg(test)] mod tests` block:

```rust
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
    component.status = ComponentStatus::Active;
    component.owner = Some("team-auth".to_string());

    store.add_component(&component).unwrap();
    store.save().unwrap();

    let store2 = LoroStore::open(tmp.path()).unwrap();
    let components = store2.list_components().unwrap();

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].base.title, "Auth Service");
    assert_eq!(components[0].status, ComponentStatus::Active);
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
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/storage/loro_store.rs
git commit -m "test: add unit tests for all entity types"
```

---

## Task 11: Run Full Test Suite and Verify

**Step 1: Run all tests**

Run: `cargo test`
Expected: All tests pass (existing + new)

**Step 2: Run manual CLI verification**

```bash
cargo build
./target/debug/medulla init
./target/debug/medulla add task "Test task" --priority=high
./target/debug/medulla add note "Test note" --type=idea
./target/debug/medulla add prompt "Test prompt" --var=input
./target/debug/medulla add component "Test API" --type=service
./target/debug/medulla add link "Docs" --url=https://example.com
./target/debug/medulla list task
./target/debug/medulla list note
./target/debug/medulla list prompt
./target/debug/medulla list component
./target/debug/medulla list link
```

Expected: All commands work without errors

**Step 3: Final commit**

```bash
git add -A
git commit -m "feat: complete Phase 1 entity types implementation"
```

---

## Summary

This plan implements the remaining 5 entity types to complete Phase 1:

| Task | Description | Files |
|------|-------------|-------|
| 1 | Task entity type | `src/entity/task.rs` |
| 2 | Note entity type | `src/entity/note.rs` |
| 3 | Prompt entity type | `src/entity/prompt.rs` |
| 4 | Component entity type | `src/entity/component.rs` |
| 5 | Link entity type | `src/entity/link.rs` |
| 6 | Task Loro storage | `src/storage/loro_store.rs` |
| 7 | Remaining Loro storage | `src/storage/loro_store.rs` |
| 8 | CLI commands | `src/cli/commands.rs` |
| 9 | CLI handlers | `src/cli/handlers.rs`, `src/main.rs` |
| 10 | Unit tests | `src/storage/loro_store.rs` |
| 11 | Verification | Full test suite |

After completion, Phase 1 will be fully complete and ready for Phase 2 MCP Server implementation.
