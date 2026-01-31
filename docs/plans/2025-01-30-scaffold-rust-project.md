# Medulla Rust Project Scaffold Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Scaffold the Medulla Rust project with a working vertical slice: `init`, `add decision`, and `list` commands that prove the Loro CRDT layer works.

**Architecture:** Single binary CLI using clap for argument parsing, Loro for CRDT storage, and a thin storage layer that handles file I/O. The vertical slice focuses on decisions only—other entity types come later.

**Tech Stack:** Rust 2021 edition, loro (CRDT), clap (CLI), serde/serde_json (serialization), uuid (entity IDs), chrono (timestamps), thiserror (error handling)

---

## Task 1: Initialize Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `.gitignore`

**Step 1: Create the cargo project**

Run: `cargo init --name medulla`

Expected: Creates `Cargo.toml` and `src/main.rs`

**Step 2: Update Cargo.toml with dependencies**

Replace `Cargo.toml` with:

```toml
[package]
name = "medulla"
version = "0.1.0"
edition = "2021"
description = "A git-native, AI-accessible knowledge engine for software projects"
license = "MIT"
repository = "https://github.com/jordanstella/medulla"

[dependencies]
# CLI
clap = { version = "4", features = ["derive"] }

# CRDT
loro = "1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "2"

[profile.release]
lto = true
strip = true
```

**Step 3: Update .gitignore**

Append to `.gitignore`:

```
# Rust
/target/
Cargo.lock

# Medulla local data
.medulla/cache.db
```

**Step 4: Verify project compiles**

Run: `cargo build`

Expected: Build succeeds with no errors

**Step 5: Commit**

```bash
git add Cargo.toml src/main.rs .gitignore
git commit -m "chore: initialize cargo project with dependencies"
```

---

## Task 2: Create Module Structure

**Files:**
- Create: `src/lib.rs`
- Create: `src/cli/mod.rs`
- Create: `src/storage/mod.rs`
- Create: `src/entity/mod.rs`
- Create: `src/error.rs`
- Modify: `src/main.rs`

**Step 1: Create error module**

Create `src/error.rs`:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MedullaError {
    #[error("Not in a medulla project. Run 'medulla init' first.")]
    NotInitialized,

    #[error("Already initialized. Remove .medulla/ to reinitialize.")]
    AlreadyInitialized,

    #[error("Entity not found: {0}")]
    EntityNotFound(String),

    #[error("Invalid entity type: {0}")]
    InvalidEntityType(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Loro error: {0}")]
    Loro(#[from] loro::LoroError),
}

pub type Result<T> = std::result::Result<T, MedullaError>;
```

**Step 2: Create entity module stub**

Create `src/entity/mod.rs`:

```rust
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
```

**Step 3: Create decision entity**

Create `src/entity/decision.rs`:

```rust
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
```

**Step 4: Create storage module stub**

Create `src/storage/mod.rs`:

```rust
mod loro_store;

pub use loro_store::LoroStore;
```

**Step 5: Create CLI module stub**

Create `src/cli/mod.rs`:

```rust
mod commands;

pub use commands::{Cli, Commands};
```

**Step 6: Create lib.rs to export modules**

Create `src/lib.rs`:

```rust
pub mod cli;
pub mod entity;
pub mod error;
pub mod storage;

pub use error::{MedullaError, Result};
```

**Step 7: Update main.rs**

Replace `src/main.rs` with:

```rust
use clap::Parser;
use medulla::cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes, no } => {
            println!("Init called with yes={}, no={}", yes, no);
            Ok(())
        }
        Commands::Add(add) => {
            println!("Add called: {:?}", add);
            Ok(())
        }
        Commands::List { entity_type, json } => {
            println!("List called: type={:?}, json={}", entity_type, json);
            Ok(())
        }
        Commands::Get { id, json } => {
            println!("Get called: id={}, json={}", id, json);
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

**Step 8: Verify project compiles**

Run: `cargo build`

Expected: Compile error (commands module doesn't exist yet)

**Step 9: Commit module structure**

```bash
git add src/
git commit -m "chore: create module structure (entity, storage, cli, error)"
```

---

## Task 3: Implement CLI Commands Structure

**Files:**
- Create: `src/cli/commands.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Create commands.rs with CLI structure**

Create `src/cli/commands.rs`:

```rust
use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "medulla")]
#[command(version, about = "A git-native, AI-accessible knowledge engine")]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new medulla project in the current directory
    Init {
        /// Accept all optional features without prompting
        #[arg(long, conflicts_with = "no")]
        yes: bool,

        /// Decline all optional features without prompting
        #[arg(long, conflicts_with = "yes")]
        no: bool,
    },

    /// Add a new entity
    Add(AddCommand),

    /// List entities
    List {
        /// Entity type to list (decision, task, note, etc.)
        #[arg(value_name = "TYPE")]
        entity_type: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Get a single entity by ID
    Get {
        /// Entity ID (sequence number like "3" or UUID prefix like "a1b2c")
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Args, Debug)]
pub struct AddCommand {
    #[command(subcommand)]
    pub entity: AddEntity,
}

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
}
```

**Step 2: Verify CLI parses correctly**

Run: `cargo build && ./target/debug/medulla --help`

Expected:
```
A git-native, AI-accessible knowledge engine

Usage: medulla <COMMAND>

Commands:
  init  Initialize a new medulla project in the current directory
  add   Add a new entity
  list  List entities
  get   Get a single entity by ID
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**Step 3: Test subcommand help**

Run: `./target/debug/medulla add decision --help`

Expected: Shows help for add decision command with all flags

**Step 4: Commit**

```bash
git add src/cli/
git commit -m "feat: implement CLI command structure with clap"
```

---

## Task 4: Implement Loro Storage Layer

**Files:**
- Create: `src/storage/loro_store.rs`
- Modify: `src/storage/mod.rs`

**Step 1: Write failing test for LoroStore**

Create `src/storage/loro_store.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

use loro::{LoroDoc, LoroMap, LoroValue};

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
                LoroValue::I64(n) => Some(n as u32),
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

        for (id_str, value) in decisions_map.iter() {
            if let LoroValue::Container(container_id) = value {
                if let Some(entity_map) = self.doc.get_by_path(&[id_str.into()]).and_then(|v| {
                    // This is a simplified approach - in production we'd properly traverse
                    None::<LoroMap>
                }) {
                    // Parse decision from map
                    // For now, we'll use a different approach
                }
            }

            // Use the JSON export approach for simplicity in v1
            let json = decisions_map.get_deep_value();
            if let LoroValue::Map(map) = json {
                for (_, entity_value) in map {
                    if let LoroValue::Map(entity_map) = entity_value {
                        if let Some(decision) = self.parse_decision_from_map(&entity_map) {
                            decisions.push(decision);
                        }
                    }
                }
            }
            break; // Only need to do this once
        }

        // Sort by sequence number
        decisions.sort_by_key(|d| d.base.sequence_number);
        Ok(decisions)
    }

    fn parse_decision_from_map(&self, map: &std::collections::HashMap<String, LoroValue>) -> Option<Decision> {
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
        let store = LoroStore::init(tmp.path()).unwrap();

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
```

**Step 2: Add tempfile dev dependency**

Add to `Cargo.toml` under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = "3"
```

**Step 3: Run tests to verify they fail appropriately then pass**

Run: `cargo test`

Expected: Tests compile and pass (or identify any issues to fix)

**Step 4: Commit**

```bash
git add Cargo.toml src/storage/
git commit -m "feat: implement Loro storage layer with add/list decisions"
```

---

## Task 5: Wire Up CLI to Storage Layer

**Files:**
- Modify: `src/main.rs`
- Create: `src/cli/handlers.rs`
- Modify: `src/cli/mod.rs`

**Step 1: Create handlers module**

Create `src/cli/handlers.rs`:

```rust
use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::entity::{Decision, DecisionStatus};
use crate::error::Result;
use crate::storage::LoroStore;

/// Find the project root by looking for .medulla/ or .git/
fn find_project_root() -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut current = cwd.as_path();
    loop {
        if current.join(".medulla").exists() || current.join(".git").exists() {
            return current.to_path_buf();
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return cwd,
        }
    }
}

pub fn handle_init(yes: bool, no: bool) -> Result<()> {
    let root = env::current_dir()?;

    let store = LoroStore::init(&root)?;

    println!("Initialized medulla project in {}", root.display());

    // For now, skip the git hook prompt (will implement in Phase 4)
    if yes {
        println!("  (git hook installation skipped - coming in Phase 4)");
    }

    Ok(())
}

pub fn handle_add_decision(
    title: String,
    status: String,
    tags: Vec<String>,
    relations: Vec<String>,
    stdin: bool,
    edit: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let seq = store.next_sequence_number("decisions");
    let mut decision = Decision::new(title, seq);

    // Parse and set status
    decision.status = status.parse().unwrap_or_default();

    // Set tags
    decision.base.tags = tags;

    // Read content from stdin if requested
    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            decision.base.content = Some(content);
        }
    }

    // TODO: Handle --edit flag (Phase 1 deferred - needs $EDITOR integration)
    if edit {
        eprintln!("Warning: --edit flag not yet implemented, skipping");
    }

    // TODO: Handle relations (Phase 1 deferred - need relations storage)
    if !relations.is_empty() {
        eprintln!("Warning: relations not yet implemented, skipping");
    }

    // Try to get git author
    decision.base.created_by = get_git_author();

    store.add_decision(&decision)?;
    store.save()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!(
            "Created decision {:03} ({}) - {}",
            decision.base.sequence_number,
            &decision.base.id.to_string()[..7],
            decision.base.title
        );
    }

    Ok(())
}

pub fn handle_list(entity_type: Option<String>, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // For now, only support decisions
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
        _ => {
            eprintln!("Entity type '{}' not yet supported. Try 'decision'.", entity_type);
        }
    }

    Ok(())
}

pub fn handle_get(id: String, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let decisions = store.list_decisions()?;

    // Try to find by sequence number first, then by UUID prefix
    let decision = if let Ok(seq) = id.parse::<u32>() {
        decisions.iter().find(|d| d.base.sequence_number == seq)
    } else {
        decisions.iter().find(|d| d.base.id.to_string().starts_with(&id))
    };

    match decision {
        Some(d) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&d)?);
            } else {
                println!("Decision {:03} ({})", d.base.sequence_number, d.base.id);
                println!("Title: {}", d.base.title);
                println!("Status: {}", d.status);
                println!("Created: {}", d.base.created_at.format("%Y-%m-%d %H:%M"));
                if let Some(ref author) = d.base.created_by {
                    println!("Author: {}", author);
                }
                if !d.base.tags.is_empty() {
                    println!("Tags: {}", d.base.tags.join(", "));
                }
                if let Some(ref content) = d.base.content {
                    println!("\n{}", content);
                }
            }
        }
        None => {
            return Err(crate::error::MedullaError::EntityNotFound(id));
        }
    }

    Ok(())
}

fn get_git_author() -> Option<String> {
    std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            } else {
                None
            }
        })
}
```

**Step 2: Update cli/mod.rs**

Replace `src/cli/mod.rs` with:

```rust
mod commands;
mod handlers;

pub use commands::{AddCommand, AddEntity, Cli, Commands};
pub use handlers::{handle_add_decision, handle_get, handle_init, handle_list};
```

**Step 3: Update main.rs to use handlers**

Replace `src/main.rs` with:

```rust
use clap::Parser;
use medulla::cli::{handle_add_decision, handle_get, handle_init, handle_list, AddEntity, Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { yes, no } => handle_init(yes, no),
        Commands::Add(add) => match add.entity {
            AddEntity::Decision {
                title,
                status,
                tags,
                relations,
                stdin,
                edit,
                json,
            } => handle_add_decision(title, status, tags, relations, stdin, edit, json),
        },
        Commands::List { entity_type, json } => handle_list(entity_type, json),
        Commands::Get { id, json } => handle_get(id, json),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
```

**Step 4: Build and test the full flow**

Run:
```bash
cargo build
cd /tmp && mkdir test-medulla && cd test-medulla
/path/to/medulla init
/path/to/medulla add decision "Use Rust for Medulla" --status=accepted --tag=architecture
/path/to/medulla add decision "Use Loro for CRDT" --status=accepted --tag=architecture
/path/to/medulla list
/path/to/medulla get 1
/path/to/medulla get 2 --json
```

Expected: All commands work, decisions are persisted and retrievable

**Step 5: Commit**

```bash
git add src/
git commit -m "feat: wire CLI to Loro storage - init, add decision, list, get working"
```

---

## Task 6: Add Integration Tests

**Files:**
- Create: `tests/integration_test.rs`

**Step 1: Create integration test**

Create `tests/integration_test.rs`:

```rust
use std::process::Command;
use tempfile::TempDir;

fn medulla_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_medulla"))
}

#[test]
fn test_init_creates_medulla_directory() {
    let tmp = TempDir::new().unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(tmp.path().join(".medulla").exists());
    assert!(tmp.path().join(".medulla/loro.db").exists());
}

#[test]
fn test_init_twice_fails() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Already initialized"));
}

#[test]
fn test_add_decision_without_init_fails() {
    let tmp = TempDir::new().unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Test"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not in a medulla project"));
}

#[test]
fn test_full_decision_workflow() {
    let tmp = TempDir::new().unwrap();

    // Init
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();
    assert!(output.status.success());

    // Add first decision
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Use Rust", "--status=accepted", "--tag=lang"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("001"));
    assert!(stdout.contains("Use Rust"));

    // Add second decision
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Use Loro", "--status=accepted"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("002"));

    // List decisions
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["list"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Use Rust"));
    assert!(stdout.contains("Use Loro"));
    assert!(stdout.contains("001"));
    assert!(stdout.contains("002"));

    // Get by sequence number
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["get", "1"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Use Rust"));
    assert!(stdout.contains("accepted"));

    // Get with JSON output
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["get", "2", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"title\": \"Use Loro\""));
}

#[test]
fn test_list_json_output() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Test Decision"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["list", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify it's valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}
```

**Step 2: Run integration tests**

Run: `cargo test --test integration_test`

Expected: All tests pass

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add integration tests for CLI workflow"
```

---

## Task 7: Final Verification and Cleanup

**Step 1: Run all tests**

Run: `cargo test`

Expected: All unit and integration tests pass

**Step 2: Run clippy for lints**

Run: `cargo clippy -- -D warnings`

Expected: No warnings

**Step 3: Format code**

Run: `cargo fmt`

**Step 4: Build release binary**

Run: `cargo build --release`

Expected: Produces optimized binary at `target/release/medulla`

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: cleanup and verify all tests pass"
```

---

## Validation Checklist

After completing all tasks, verify:

- [ ] `medulla init` creates `.medulla/loro.db`
- [ ] `medulla add decision "Title"` creates a decision with sequence number 001
- [ ] `medulla list` shows all decisions sorted by sequence number
- [ ] `medulla get 1` retrieves decision by sequence number
- [ ] `medulla get <uuid-prefix>` retrieves decision by UUID prefix
- [ ] `--json` flag works on all read commands
- [ ] Data persists after reopening (close and reopen)
- [ ] `cargo test` passes all tests
- [ ] Binary size is reasonable (< 10MB release build)
