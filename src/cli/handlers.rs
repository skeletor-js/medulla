use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Duration;

use crate::cache::SqliteCache;
use crate::embeddings::Embedder;
use crate::entity::{
    Component, ComponentStatus, Decision, DecisionStatus, Link, Note, Prompt, Relation,
    RelationType, Task, TaskStatus,
};
use crate::error::{MedullaError, Result};
use crate::mcp::MedullaServer;
use std::sync::OnceLock;

/// Lazy-initialized embedding model for CLI.
static CLI_EMBEDDER: OnceLock<std::result::Result<Embedder, String>> = OnceLock::new();

/// Get the embedder for CLI operations.
fn get_embedder() -> Option<&'static Embedder> {
    let result = CLI_EMBEDDER.get_or_init(|| Embedder::new().map_err(|e| e.to_string()));
    result.as_ref().ok()
}
use crate::storage::{
    ComponentUpdate, DecisionUpdate, LinkUpdate, LoroStore, NoteUpdate, PromptUpdate, TaskUpdate,
};

/// Reference to any entity type in the system
#[derive(Clone)]
enum EntityRef {
    Decision(Decision),
    Task(Task),
    Note(Note),
    Prompt(Prompt),
    Component(Component),
    Link(Link),
}

/// Find an entity by ID (sequence number or UUID prefix) across all entity types
fn find_entity_by_id(store: &LoroStore, id: &str) -> Result<EntityRef> {
    // Try to parse as sequence number first
    if let Ok(seq) = id.parse::<u32>() {
        // Search through all entity types by sequence number
        for decision in store.list_decisions()? {
            if decision.base.sequence_number == seq {
                return Ok(EntityRef::Decision(decision));
            }
        }
        for task in store.list_tasks()? {
            if task.base.sequence_number == seq {
                return Ok(EntityRef::Task(task));
            }
        }
        for note in store.list_notes()? {
            if note.base.sequence_number == seq {
                return Ok(EntityRef::Note(note));
            }
        }
        for prompt in store.list_prompts()? {
            if prompt.base.sequence_number == seq {
                return Ok(EntityRef::Prompt(prompt));
            }
        }
        for component in store.list_components()? {
            if component.base.sequence_number == seq {
                return Ok(EntityRef::Component(component));
            }
        }
        for link in store.list_links()? {
            if link.base.sequence_number == seq {
                return Ok(EntityRef::Link(link));
            }
        }
    } else {
        // Search by UUID prefix
        for decision in store.list_decisions()? {
            if decision.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Decision(decision));
            }
        }
        for task in store.list_tasks()? {
            if task.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Task(task));
            }
        }
        for note in store.list_notes()? {
            if note.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Note(note));
            }
        }
        for prompt in store.list_prompts()? {
            if prompt.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Prompt(prompt));
            }
        }
        for component in store.list_components()? {
            if component.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Component(component));
            }
        }
        for link in store.list_links()? {
            if link.base.id.to_string().starts_with(id) {
                return Ok(EntityRef::Link(link));
            }
        }
    }

    Err(MedullaError::EntityNotFound(id.to_string()))
}

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

pub fn handle_init(yes: bool, _no: bool) -> Result<()> {
    let root = env::current_dir()?;

    let _store = LoroStore::init(&root)?;

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

    let seq = store.next_sequence_number();
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

    // TODO: Handle --edit flag (Phase 4 - needs $EDITOR integration)
    if edit {
        eprintln!("Warning: --edit flag not yet implemented, skipping");
    }

    // Try to get git author
    let git_author = get_git_author();
    decision.base.created_by = git_author.clone();

    store.add_decision(&decision)?;

    // Handle relations after decision is added
    for rel_str in &relations {
        match parse_relation_string(rel_str) {
            Ok((rel_type, target_id)) => {
                let mut relation = Relation::new(
                    decision.base.id,
                    "decision".to_string(),
                    target_id,
                    "unknown".to_string(), // Will be resolved when target entity types are implemented
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

    let seq = store.next_sequence_number();
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

    let seq = store.next_sequence_number();
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

    let seq = store.next_sequence_number();
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

    let seq = store.next_sequence_number();
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
    add_relations_for_entity(
        &store,
        component.base.id,
        "component",
        &relations,
        &git_author,
    )?;
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

    let seq = store.next_sequence_number();
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
                    let due_str = t
                        .due_date
                        .map(|d| format!(" due:{}", d))
                        .unwrap_or_default();
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

pub fn handle_get(id: String, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let decisions = store.list_decisions()?;

    // Try to find by sequence number first, then by UUID prefix
    let decision = if let Ok(seq) = id.parse::<u32>() {
        decisions.iter().find(|d| d.base.sequence_number == seq)
    } else {
        decisions
            .iter()
            .find(|d| d.base.id.to_string().starts_with(&id))
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

/// Parse a relation string in format "type:target_id"
fn parse_relation_string(s: &str) -> Result<(RelationType, uuid::Uuid)> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(MedullaError::Storage(format!(
            "Invalid relation format '{}'. Expected 'type:target_id'",
            s
        )));
    }

    let rel_type: RelationType = parts[0]
        .parse()
        .map_err(|e: String| MedullaError::Storage(e))?;

    let target_id = uuid::Uuid::parse_str(parts[1])
        .map_err(|_| MedullaError::Storage(format!("Invalid UUID in relation: {}", parts[1])))?;

    Ok((rel_type, target_id))
}

pub fn handle_update(
    id: String,
    title: Option<String>,
    status: Option<String>,
    tags: Vec<String>,
    remove_tags: Vec<String>,
    relations: Vec<String>,
    stdin: bool,
    edit: bool,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Find the entity by ID across all types
    let entity = find_entity_by_id(&store, &id)?;

    // TODO: Handle --edit flag (Phase 4)
    if edit {
        eprintln!("Warning: --edit flag not yet implemented, skipping");
    }

    let git_author = get_git_author();

    // Handle updates based on entity type
    match entity {
        EntityRef::Decision(decision) => {
            let mut updates = DecisionUpdate::default();
            updates.title = title;
            updates.status = status.and_then(|s| s.parse::<DecisionStatus>().ok());
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_decision(&decision.base.id, updates)?;
            add_relations_for_entity(
                &store,
                decision.base.id,
                "decision",
                &relations,
                &git_author,
            )?;
            store.save()?;

            let updated = store.get_decision(&decision.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated decision".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated decision {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
        EntityRef::Task(task) => {
            let mut updates = TaskUpdate::default();
            updates.title = title;
            updates.status = status.and_then(|s| s.parse::<TaskStatus>().ok());
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_task(&task.base.id, updates)?;
            add_relations_for_entity(&store, task.base.id, "task", &relations, &git_author)?;
            store.save()?;

            let updated = store.get_task(&task.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated task".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated task {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
        EntityRef::Note(note) => {
            let mut updates = NoteUpdate::default();
            updates.title = title;
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_note(&note.base.id, updates)?;
            add_relations_for_entity(&store, note.base.id, "note", &relations, &git_author)?;
            store.save()?;

            let updated = store.get_note(&note.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated note".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated note {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
        EntityRef::Prompt(prompt) => {
            let mut updates = PromptUpdate::default();
            updates.title = title;
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_prompt(&prompt.base.id, updates)?;
            add_relations_for_entity(&store, prompt.base.id, "prompt", &relations, &git_author)?;
            store.save()?;

            let updated = store.get_prompt(&prompt.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated prompt".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated prompt {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
        EntityRef::Component(component) => {
            let mut updates = ComponentUpdate::default();
            updates.title = title;
            updates.status = status.and_then(|s| s.parse::<ComponentStatus>().ok());
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_component(&component.base.id, updates)?;
            add_relations_for_entity(
                &store,
                component.base.id,
                "component",
                &relations,
                &git_author,
            )?;
            store.save()?;

            let updated = store.get_component(&component.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated component".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated component {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
        EntityRef::Link(link) => {
            let mut updates = LinkUpdate::default();
            updates.title = title;
            updates.add_tags = tags;
            updates.remove_tags = remove_tags;

            if stdin {
                let mut content = String::new();
                io::stdin().read_to_string(&mut content)?;
                if !content.is_empty() {
                    updates.content = Some(content);
                }
            }

            store.update_link(&link.base.id, updates)?;
            add_relations_for_entity(&store, link.base.id, "link", &relations, &git_author)?;
            store.save()?;

            let updated = store.get_link(&link.base.id)?.ok_or_else(|| {
                MedullaError::Storage("Failed to retrieve updated link".to_string())
            })?;

            if json {
                println!("{}", serde_json::to_string_pretty(&updated)?);
            } else {
                println!(
                    "Updated link {:03} ({}) - {}",
                    updated.base.sequence_number,
                    &updated.base.id.to_string()[..7],
                    updated.base.title
                );
            }
        }
    }

    Ok(())
}

pub fn handle_delete(id: String, force: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Find the entity by ID across all types
    let entity = find_entity_by_id(&store, &id)?;

    // Extract entity info for confirmation message
    let (entity_id, entity_type, title, sequence_number) = match &entity {
        EntityRef::Decision(d) => (
            d.base.id,
            "decision",
            d.base.title.clone(),
            d.base.sequence_number,
        ),
        EntityRef::Task(t) => (
            t.base.id,
            "task",
            t.base.title.clone(),
            t.base.sequence_number,
        ),
        EntityRef::Note(n) => (
            n.base.id,
            "note",
            n.base.title.clone(),
            n.base.sequence_number,
        ),
        EntityRef::Prompt(p) => (
            p.base.id,
            "prompt",
            p.base.title.clone(),
            p.base.sequence_number,
        ),
        EntityRef::Component(c) => (
            c.base.id,
            "component",
            c.base.title.clone(),
            c.base.sequence_number,
        ),
        EntityRef::Link(l) => (
            l.base.id,
            "link",
            l.base.title.clone(),
            l.base.sequence_number,
        ),
    };

    // Confirm deletion unless --force is used
    if !force {
        eprintln!(
            "Delete {} {:03} ({}) - {}? [y/N] ",
            entity_type,
            sequence_number,
            &entity_id.to_string()[..7],
            title
        );

        // Check if stdin is a tty for interactive confirmation
        if atty::is(atty::Stream::Stdin) {
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled.");
                return Ok(());
            }
        } else {
            // Non-interactive mode without --force, abort
            return Err(MedullaError::Storage(
                "Use --force to delete in non-interactive mode".to_string(),
            ));
        }
    }

    // Delete any relations involving this entity
    let relations = store.list_relations()?;
    for rel in relations {
        if rel.source_id == entity_id || rel.target_id == entity_id {
            let _ = store.delete_relation(
                &rel.source_id.to_string(),
                &rel.relation_type.to_string(),
                &rel.target_id.to_string(),
            );
        }
    }

    // Delete the entity based on its type
    match entity {
        EntityRef::Decision(_) => store.delete_decision(&entity_id)?,
        EntityRef::Task(_) => store.delete_task(&entity_id)?,
        EntityRef::Note(_) => store.delete_note(&entity_id)?,
        EntityRef::Prompt(_) => store.delete_prompt(&entity_id)?,
        EntityRef::Component(_) => store.delete_component(&entity_id)?,
        EntityRef::Link(_) => store.delete_link(&entity_id)?,
    }

    store.save()?;

    println!(
        "Deleted {} {:03} ({}) - {}",
        entity_type,
        sequence_number,
        &entity_id.to_string()[..7],
        title
    );

    Ok(())
}

pub fn handle_tasks_ready(limit: u32, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    let ready_tasks = cache.get_ready_tasks(Some(limit))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&ready_tasks)?);
    } else if ready_tasks.is_empty() {
        println!("No ready tasks found.");
    } else {
        println!("Ready tasks ({}): \n", ready_tasks.len());
        for task in ready_tasks {
            let due_str = task
                .due_date
                .as_ref()
                .map(|d| format!(" due:{}", d))
                .unwrap_or_default();
            let assignee_str = task
                .assignee
                .as_ref()
                .map(|a| format!(" @{}", a))
                .unwrap_or_default();
            println!(
                "  {:03} ({}) [{}|{}]{}{} {}",
                task.sequence_number,
                &task.id[..7.min(task.id.len())],
                task.status,
                task.priority,
                due_str,
                assignee_str,
                task.title
            );
        }
    }

    Ok(())
}

pub fn handle_tasks_next(json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    let next_task = cache.get_next_task()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&next_task)?);
    } else if let Some(task) = next_task {
        let due_str = task
            .due_date
            .as_ref()
            .map(|d| format!(" due:{}", d))
            .unwrap_or_default();
        let assignee_str = task
            .assignee
            .as_ref()
            .map(|a| format!(" @{}", a))
            .unwrap_or_default();
        println!("Next task:\n");
        println!(
            "  {:03} ({}) [{}|{}]{}{} {}",
            task.sequence_number,
            &task.id[..7.min(task.id.len())],
            task.status,
            task.priority,
            due_str,
            assignee_str,
            task.title
        );
    } else {
        println!("No ready tasks found.");
    }

    Ok(())
}

pub fn handle_tasks_blocked(id: Option<String>, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    if let Some(task_id) = id {
        // Show blockers for a specific task
        // First, resolve the task ID (could be sequence number or UUID prefix)
        let resolved_id = resolve_task_id(&store, &task_id)?;
        let blockers = cache.get_task_blockers(&resolved_id)?;

        if json {
            println!("{}", serde_json::to_string_pretty(&blockers)?);
        } else if blockers.is_empty() {
            println!("Task {} has no blockers.", task_id);
        } else {
            println!("Blockers for task {}:\n", task_id);
            for blocker in blockers {
                println!(
                    "  {:03} ({}) [{}] {}",
                    blocker.sequence_number,
                    &blocker.id[..7.min(blocker.id.len())],
                    blocker.status,
                    blocker.title
                );
            }
        }
    } else {
        // Show all blocked tasks
        let blocked_tasks = cache.get_blocked_tasks(None)?;

        if json {
            println!("{}", serde_json::to_string_pretty(&blocked_tasks)?);
        } else if blocked_tasks.is_empty() {
            println!("No blocked tasks found.");
        } else {
            println!("Blocked tasks ({}):\n", blocked_tasks.len());
            for task in blocked_tasks {
                let due_str = task
                    .due_date
                    .as_ref()
                    .map(|d| format!(" due:{}", d))
                    .unwrap_or_default();
                println!(
                    "  {:03} ({}) [{}|{}]{} {}",
                    task.sequence_number,
                    &task.id[..7.min(task.id.len())],
                    task.status,
                    task.priority,
                    due_str,
                    task.title
                );
                println!("      blocked by:");
                for blocker in &task.blockers {
                    println!(
                        "        - {:03} ({}) [{}] {}",
                        blocker.sequence_number,
                        &blocker.id[..7.min(blocker.id.len())],
                        blocker.status,
                        blocker.title
                    );
                }
            }
        }
    }

    Ok(())
}

/// Resolve a task ID from sequence number or UUID prefix to full UUID
fn resolve_task_id(store: &LoroStore, id: &str) -> Result<String> {
    // Try to parse as sequence number first
    if let Ok(seq) = id.parse::<u32>() {
        for task in store.list_tasks()? {
            if task.base.sequence_number == seq {
                return Ok(task.base.id.to_string());
            }
        }
    } else {
        // Search by UUID prefix
        for task in store.list_tasks()? {
            if task.base.id.to_string().starts_with(id) {
                return Ok(task.base.id.to_string());
            }
        }
    }

    Err(MedullaError::EntityNotFound(id.to_string()))
}

/// Find an entity by ID and return its UUID and type
fn find_entity_id_with_type(store: &LoroStore, id: &str) -> Result<(uuid::Uuid, String)> {
    let entity = find_entity_by_id(store, id)?;
    match entity {
        EntityRef::Decision(d) => Ok((d.base.id, "decision".to_string())),
        EntityRef::Task(t) => Ok((t.base.id, "task".to_string())),
        EntityRef::Note(n) => Ok((n.base.id, "note".to_string())),
        EntityRef::Prompt(p) => Ok((p.base.id, "prompt".to_string())),
        EntityRef::Component(c) => Ok((c.base.id, "component".to_string())),
        EntityRef::Link(l) => Ok((l.base.id, "link".to_string())),
    }
}

/// Get entity title by ID for display purposes
fn get_entity_title(store: &LoroStore, id: &uuid::Uuid) -> String {
    // Search through all entity types
    if let Ok(decisions) = store.list_decisions() {
        for d in decisions {
            if d.base.id == *id {
                return d.base.title;
            }
        }
    }
    if let Ok(tasks) = store.list_tasks() {
        for t in tasks {
            if t.base.id == *id {
                return t.base.title;
            }
        }
    }
    if let Ok(notes) = store.list_notes() {
        for n in notes {
            if n.base.id == *id {
                return n.base.title;
            }
        }
    }
    if let Ok(prompts) = store.list_prompts() {
        for p in prompts {
            if p.base.id == *id {
                return p.base.title;
            }
        }
    }
    if let Ok(components) = store.list_components() {
        for c in components {
            if c.base.id == *id {
                return c.base.title;
            }
        }
    }
    if let Ok(links) = store.list_links() {
        for l in links {
            if l.base.id == *id {
                return l.base.title;
            }
        }
    }
    id.to_string()[..7].to_string()
}

pub fn handle_relation_add(
    source_id: String,
    target_id: String,
    relation_type: String,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Resolve source and target IDs to UUIDs with types
    let (source_uuid, source_type) = find_entity_id_with_type(&store, &source_id)?;
    let (target_uuid, target_type) = find_entity_id_with_type(&store, &target_id)?;

    // Parse and validate relation type
    let rel_type: RelationType = relation_type
        .parse()
        .map_err(|e: String| MedullaError::Storage(e))?;

    // Create the relation
    let mut relation = Relation::new(
        source_uuid,
        source_type.clone(),
        target_uuid,
        target_type.clone(),
        rel_type.clone(),
    );

    // Try to get git author
    let git_author = get_git_author();
    relation.created_by = git_author;

    // Store the relation
    store.add_relation(&relation)?;
    store.save()?;

    if json {
        let response = serde_json::json!({
            "source_id": source_uuid.to_string(),
            "source_type": source_type,
            "target_id": target_uuid.to_string(),
            "target_type": target_type,
            "relation_type": rel_type.to_string(),
            "created_at": relation.created_at.to_rfc3339(),
            "message": format!(
                "Created '{}' relation from {} to {}",
                rel_type, source_id, target_id
            ),
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Created '{}' relation: {} ({}) -> {} ({})",
            rel_type,
            source_id,
            source_type,
            target_id,
            target_type
        );
    }

    Ok(())
}

pub fn handle_relation_delete(
    source_id: String,
    target_id: String,
    relation_type: String,
    json: bool,
) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Resolve source and target IDs to UUIDs
    let (source_uuid, _) = find_entity_id_with_type(&store, &source_id)?;
    let (target_uuid, _) = find_entity_id_with_type(&store, &target_id)?;

    // Parse and validate relation type
    let rel_type: RelationType = relation_type
        .parse()
        .map_err(|e: String| MedullaError::Storage(e))?;

    // Delete from store
    store.delete_relation(
        &source_uuid.to_string(),
        &rel_type.to_string(),
        &target_uuid.to_string(),
    )?;
    store.save()?;

    if json {
        let response = serde_json::json!({
            "deleted": true,
            "source_id": source_uuid.to_string(),
            "target_id": target_uuid.to_string(),
            "relation_type": rel_type.to_string(),
            "message": format!(
                "Deleted '{}' relation from {} to {}",
                rel_type, source_id, target_id
            ),
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        println!(
            "Deleted '{}' relation: {} -> {}",
            rel_type, source_id, target_id
        );
    }

    Ok(())
}

pub fn handle_relation_list(entity_id: String, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Resolve entity ID to UUID
    let (entity_uuid, entity_type) = find_entity_id_with_type(&store, &entity_id)?;

    // Get all relations involving this entity (both as source and target)
    let relations_from = store.get_relations_from(&entity_uuid.to_string())?;
    let relations_to = store.get_relations_to(&entity_uuid.to_string())?;

    if json {
        #[derive(serde::Serialize)]
        struct RelationJson {
            direction: String,
            relation_type: String,
            other_id: String,
            other_type: String,
            other_title: String,
            created_at: String,
        }

        let mut all_relations: Vec<RelationJson> = Vec::new();

        for r in &relations_from {
            all_relations.push(RelationJson {
                direction: "outgoing".to_string(),
                relation_type: r.relation_type.to_string(),
                other_id: r.target_id.to_string(),
                other_type: r.target_type.clone(),
                other_title: get_entity_title(&store, &r.target_id),
                created_at: r.created_at.to_rfc3339(),
            });
        }

        for r in &relations_to {
            all_relations.push(RelationJson {
                direction: "incoming".to_string(),
                relation_type: r.relation_type.to_string(),
                other_id: r.source_id.to_string(),
                other_type: r.source_type.clone(),
                other_title: get_entity_title(&store, &r.source_id),
                created_at: r.created_at.to_rfc3339(),
            });
        }

        let response = serde_json::json!({
            "entity_id": entity_uuid.to_string(),
            "entity_type": entity_type,
            "relations": all_relations,
        });
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let entity_title = get_entity_title(&store, &entity_uuid);
        println!(
            "Relations for {} ({}) - {}:\n",
            entity_id, entity_type, entity_title
        );

        if relations_from.is_empty() && relations_to.is_empty() {
            println!("  No relations found.");
        } else {
            if !relations_from.is_empty() {
                println!("  Outgoing relations:");
                for r in &relations_from {
                    let target_title = get_entity_title(&store, &r.target_id);
                    println!(
                        "    --[{}]--> {} ({}) - {}",
                        r.relation_type,
                        &r.target_id.to_string()[..7],
                        r.target_type,
                        target_title
                    );
                }
            }

            if !relations_to.is_empty() {
                println!("  Incoming relations:");
                for r in &relations_to {
                    let source_title = get_entity_title(&store, &r.source_id);
                    println!(
                        "    <--[{}]-- {} ({}) - {}",
                        r.relation_type,
                        &r.source_id.to_string()[..7],
                        r.source_type,
                        source_title
                    );
                }
            }
        }
    }

    Ok(())
}

pub fn handle_search(query: String, semantic: bool, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    // Parse query for filters (type:, status:, tag:, created:)
    let (search_text, filter) = crate::search::parse_query(&query);

    if semantic {
        return handle_search_semantic(&cache, &search_text, &filter, json);
    }

    // Determine search text (if empty after parsing, search all)
    let search_query = if search_text.is_empty() { "*" } else { &search_text };

    // Perform full-text search across all entity types (or filtered type)
    let results = if let Some(ref entity_type) = filter.entity_type {
        // Search only the specified entity type
        cache.search_by_type(entity_type, search_query, 100)?
    } else {
        cache.search_all(search_query, 100)?
    };

    // Apply additional filters
    let results: Vec<_> = results
        .into_iter()
        .filter(|r| matches_cli_filter(&cache, r, &filter))
        .take(50)
        .collect();

    if json {
        #[derive(serde::Serialize)]
        struct SearchResultJson {
            entity_type: String,
            id: String,
            sequence_number: u32,
            title: String,
            status: Option<String>,
            snippet: Option<String>,
        }

        let json_results: Vec<SearchResultJson> = results
            .into_iter()
            .map(|r| match r {
                crate::cache::SearchResult::Decision(d) => SearchResultJson {
                    entity_type: "decision".to_string(),
                    id: d.id,
                    sequence_number: d.sequence_number,
                    title: d.title,
                    status: Some(d.status),
                    snippet: d.content_snippet,
                },
                crate::cache::SearchResult::Task(t) => SearchResultJson {
                    entity_type: "task".to_string(),
                    id: t.id,
                    sequence_number: t.sequence_number,
                    title: t.title,
                    status: Some(t.status),
                    snippet: t.content_snippet,
                },
                crate::cache::SearchResult::Note(n) => SearchResultJson {
                    entity_type: "note".to_string(),
                    id: n.id,
                    sequence_number: n.sequence_number,
                    title: n.title,
                    status: n.note_type,
                    snippet: n.content_snippet,
                },
                crate::cache::SearchResult::Prompt(p) => SearchResultJson {
                    entity_type: "prompt".to_string(),
                    id: p.id,
                    sequence_number: p.sequence_number,
                    title: p.title,
                    status: None,
                    snippet: p.content_snippet,
                },
                crate::cache::SearchResult::Component(c) => SearchResultJson {
                    entity_type: "component".to_string(),
                    id: c.id,
                    sequence_number: c.sequence_number,
                    title: c.title,
                    status: Some(c.status),
                    snippet: c.content_snippet,
                },
                crate::cache::SearchResult::Link(l) => SearchResultJson {
                    entity_type: "link".to_string(),
                    id: l.id,
                    sequence_number: l.sequence_number,
                    title: l.title,
                    status: l.link_type,
                    snippet: l.content_snippet,
                },
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if results.is_empty() {
        println!("No results found for '{}'.", query);
    } else {
        println!("Search results for '{}':\n", query);
        for r in results {
            match r {
                crate::cache::SearchResult::Decision(d) => {
                    println!(
                        "  [DECISION] {:03} ({}) [{}] {}",
                        d.sequence_number,
                        &d.id[..7.min(d.id.len())],
                        d.status,
                        d.title
                    );
                    if let Some(snippet) = d.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
                crate::cache::SearchResult::Task(t) => {
                    println!(
                        "  [TASK] {:03} ({}) [{}|{}] {}",
                        t.sequence_number,
                        &t.id[..7.min(t.id.len())],
                        t.status,
                        t.priority,
                        t.title
                    );
                    if let Some(assignee) = t.assignee {
                        println!("      assignee: {}", assignee);
                    }
                    if let Some(snippet) = t.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
                crate::cache::SearchResult::Note(n) => {
                    let type_str = n.note_type.as_deref().unwrap_or("note");
                    println!(
                        "  [NOTE] {:03} ({}) [{}] {}",
                        n.sequence_number,
                        &n.id[..7.min(n.id.len())],
                        type_str,
                        n.title
                    );
                    if let Some(snippet) = n.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
                crate::cache::SearchResult::Prompt(p) => {
                    println!(
                        "  [PROMPT] {:03} ({}) {}",
                        p.sequence_number,
                        &p.id[..7.min(p.id.len())],
                        p.title
                    );
                    if !p.variables.is_empty() {
                        println!("      vars: {}", p.variables.join(", "));
                    }
                    if let Some(snippet) = p.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
                crate::cache::SearchResult::Component(c) => {
                    let type_str = c.component_type.as_deref().unwrap_or("component");
                    println!(
                        "  [COMPONENT] {:03} ({}) [{}|{}] {}",
                        c.sequence_number,
                        &c.id[..7.min(c.id.len())],
                        type_str,
                        c.status,
                        c.title
                    );
                    if let Some(owner) = c.owner {
                        println!("      owner: {}", owner);
                    }
                    if let Some(snippet) = c.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
                crate::cache::SearchResult::Link(l) => {
                    println!(
                        "  [LINK] {:03} ({}) {} -> {}",
                        l.sequence_number,
                        &l.id[..7.min(l.id.len())],
                        l.title,
                        l.url
                    );
                    if let Some(snippet) = l.content_snippet {
                        let clean_snippet = snippet
                            .replace("<mark>", "\x1b[1m")
                            .replace("</mark>", "\x1b[0m");
                        println!("      {}", clean_snippet);
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a fulltext search result matches the CLI filter.
fn matches_cli_filter(
    cache: &SqliteCache,
    result: &crate::cache::SearchResult,
    filter: &crate::search::SearchFilter,
) -> bool {
    // Get entity ID and type from result
    let (entity_id, entity_type, result_status) = match result {
        crate::cache::SearchResult::Decision(d) => (&d.id, "decision", Some(&d.status)),
        crate::cache::SearchResult::Task(t) => (&t.id, "task", Some(&t.status)),
        crate::cache::SearchResult::Component(c) => (&c.id, "component", Some(&c.status)),
        crate::cache::SearchResult::Note(n) => (&n.id, "note", None),
        crate::cache::SearchResult::Prompt(p) => (&p.id, "prompt", None),
        crate::cache::SearchResult::Link(l) => (&l.id, "link", None),
    };

    // Check status filter (use status from search result for efficiency)
    if let Some(ref required_status) = filter.status {
        match result_status {
            Some(actual) if actual == required_status => {}
            _ => return false,
        }
    }

    // If no tag or date filters, we're done
    if filter.tags.is_empty() && filter.created_after.is_none() && filter.created_before.is_none() {
        return true;
    }

    // Load full metadata for tag and date checks
    let metadata = match cache.get_filter_metadata(entity_id, entity_type) {
        Ok(Some(m)) => m,
        _ => return false,
    };

    // Check tags
    for required_tag in &filter.tags {
        if !metadata.tags.iter().any(|t| t.eq_ignore_ascii_case(required_tag)) {
            return false;
        }
    }

    // Check dates
    if let Some(ref after) = filter.created_after {
        match &metadata.created_at {
            Some(created) if created >= after => {}
            _ => return false,
        }
    }

    if let Some(ref before) = filter.created_before {
        match &metadata.created_at {
            Some(created) if created <= before => {}
            _ => return false,
        }
    }

    true
}

/// Handle semantic search using vector embeddings.
fn handle_search_semantic(
    cache: &SqliteCache,
    query: &str,
    filter: &crate::search::SearchFilter,
    json: bool,
) -> Result<()> {
    let embedder = get_embedder().ok_or_else(|| {
        MedullaError::Embedding("Embedding model not available. Try again later.".to_string())
    })?;

    // Compute query embedding
    let query_embedding = embedder.embed(query)?;

    // Perform semantic search with entity type filter
    let results = cache.search_semantic(
        &query_embedding,
        filter.entity_type.as_deref(),
        50,
        0.3,
    )?;

    // Apply additional filters (status, tags, dates)
    let results: Vec<_> = results
        .into_iter()
        .filter(|r| matches_semantic_filter(cache, r, filter))
        .take(20)
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else if results.is_empty() {
        println!("No semantically similar results found for '{}'.", query);
        println!("\nHint: Semantic search requires entities to have embeddings.");
        println!("Try running 'medulla cache rebuild' to generate embeddings for existing entities.");
    } else {
        println!("Semantic search results for '{}' (similarity threshold: 0.3):\n", query);
        for r in results {
            let type_upper = r.entity_type.to_uppercase();
            println!(
                "  [{type_upper}] {:03} ({}) {:.2}% - {}",
                r.sequence_number,
                &r.entity_id[..7.min(r.entity_id.len())],
                r.score * 100.0,
                r.title
            );
        }
    }

    Ok(())
}

/// Check if a semantic search result matches the CLI filter.
fn matches_semantic_filter(
    cache: &SqliteCache,
    result: &crate::cache::SemanticSearchResult,
    filter: &crate::search::SearchFilter,
) -> bool {
    // If no advanced filters, always match
    if filter.status.is_none()
        && filter.tags.is_empty()
        && filter.created_after.is_none()
        && filter.created_before.is_none()
    {
        return true;
    }

    // Load filter metadata
    let metadata = match cache.get_filter_metadata(&result.entity_id, &result.entity_type) {
        Ok(Some(m)) => m,
        _ => return false,
    };

    // Check status
    if let Some(ref required_status) = filter.status {
        match &metadata.status {
            Some(actual) if actual == required_status => {}
            _ => return false,
        }
    }

    // Check tags
    for required_tag in &filter.tags {
        if !metadata.tags.iter().any(|t| t.eq_ignore_ascii_case(required_tag)) {
            return false;
        }
    }

    // Check dates
    if let Some(ref after) = filter.created_after {
        match &metadata.created_at {
            Some(created) if created >= after => {}
            _ => return false,
        }
    }

    if let Some(ref before) = filter.created_before {
        match &metadata.created_at {
            Some(created) if created <= before => {}
            _ => return false,
        }
    }

    true
}

/// Start the MCP server with graceful shutdown support.
///
/// Server startup flow:
/// 1. Open existing `LoroStore` (fail if not initialized)
/// 2. Open/create `SqliteCache`, sync from Loro
/// 3. Create `MedullaServer` with store + cache
/// 4. Install signal handlers for graceful shutdown
/// 5. Call `server.serve(rmcp::transport::io::stdio()).await`
/// 6. Wait for shutdown signal
pub fn handle_serve() -> Result<()> {
    let root = find_project_root();

    // Check if this is an initialized medulla project
    if !root.join(".medulla").exists() {
        return Err(MedullaError::Storage(
            "Not a medulla project. Run `medulla init` first.".to_string(),
        ));
    }

    // Set up tracing to stderr (stdout is reserved for MCP protocol)
    let filter = tracing_subscriber::EnvFilter::try_from_env("MEDULLA_LOG_LEVEL")
        .or_else(|_| tracing_subscriber::EnvFilter::try_from_env("RUST_LOG"))
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    // Open the store and cache
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    // Check performance thresholds
    if let Ok(stats) = cache.get_stats() {
        let loro_size = std::fs::metadata(root.join(".medulla/loro.db"))
            .map(|m| m.len())
            .unwrap_or(0);
        for warning in crate::warnings::check_thresholds(&stats, loro_size) {
            tracing::warn!("{}", crate::warnings::format_warning(&warning));
        }
    }

    tracing::info!("Starting Medulla MCP server");

    // Create the server
    let server = MedullaServer::new(store, cache);

    // Run the async server with tokio runtime
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| MedullaError::Storage(format!("Failed to create tokio runtime: {}", e)))?;

    rt.block_on(async move {
        run_server(server).await
    })
}

/// Run the MCP server with graceful shutdown on SIGINT/SIGTERM.
async fn run_server(server: MedullaServer) -> Result<()> {
    use rmcp::transport::io::stdio;

    let transport = stdio();

    // Spawn the server task
    let server_task = tokio::spawn(async move {
        match server.serve(transport).await {
            Ok(_) => {
                tracing::info!("Server stopped");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Server error: {}", e);
                Err(MedullaError::Storage(format!("MCP server error: {}", e)))
            }
        }
    });

    // Wait for shutdown signal or server completion
    tokio::select! {
        result = server_task => {
            match result {
                Ok(inner_result) => inner_result,
                Err(e) => Err(MedullaError::Storage(format!("Server task panicked: {}", e))),
            }
        }
        _ = shutdown_signal() => {
            tracing::info!("Shutdown signal received, stopping server...");
            // Give a brief moment for cleanup
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(())
        }
    }
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Handle cache stats command.
pub fn handle_cache_stats(json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    let stats = cache.get_stats()?;

    // Get loro.db size
    let loro_path = root.join(".medulla/loro.db");
    let loro_size = std::fs::metadata(&loro_path)
        .map(|m| m.len())
        .unwrap_or(0);

    // Check for warnings
    let warnings = crate::warnings::check_thresholds(&stats, loro_size);

    if json {
        #[derive(serde::Serialize)]
        struct StatsJson {
            entity_count: usize,
            embedding_count: usize,
            decisions: usize,
            tasks: usize,
            notes: usize,
            prompts: usize,
            components: usize,
            links: usize,
            relations: usize,
            loro_db_size_bytes: u64,
            warnings: Vec<String>,
        }

        let json_out = StatsJson {
            entity_count: stats.entity_count,
            embedding_count: stats.embedding_count,
            decisions: stats.decisions,
            tasks: stats.tasks,
            notes: stats.notes,
            prompts: stats.prompts,
            components: stats.components,
            links: stats.links,
            relations: stats.relations,
            loro_db_size_bytes: loro_size,
            warnings: warnings
                .iter()
                .map(crate::warnings::format_warning)
                .collect(),
        };

        println!("{}", serde_json::to_string_pretty(&json_out)?);
    } else {
        println!("Cache Statistics:");
        println!("  Entities: {}", stats.entity_count);
        println!("    Decisions:  {}", stats.decisions);
        println!("    Tasks:      {}", stats.tasks);
        println!("    Notes:      {}", stats.notes);
        println!("    Prompts:    {}", stats.prompts);
        println!("    Components: {}", stats.components);
        println!("    Links:      {}", stats.links);
        println!("  Relations: {}", stats.relations);
        println!("  Embeddings: {}", stats.embedding_count);
        println!(
            "  loro.db size: {:.2} MB",
            loro_size as f64 / (1024.0 * 1024.0)
        );

        if !warnings.is_empty() {
            println!();
            for warning in &warnings {
                eprintln!("{}", crate::warnings::format_warning(warning));
            }
        }
    }

    Ok(())
}

/// Handle cache rebuild command.
pub fn handle_cache_rebuild(json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Clear the cache
    cache.clear()?;

    // Sync from store (rebuilds all index data)
    store.sync_cache(&cache)?;

    // Recompute all embeddings
    let embedder = get_embedder();
    if embedder.is_none() {
        if !json {
            eprintln!("Warning: Embedding model not available, skipping embedding regeneration");
        }
    }

    let mut embedding_count = 0;
    let mut errors = 0;

    if let Some(embedder) = embedder {
        // Process all entity types
        // Each entity type is processed with (id, title, content, tags)
        // Content comes from base.content for all types
        for (entity_type, entities) in [
            ("decision", store.list_decisions()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.base.content.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
            ("task", store.list_tasks()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.base.content.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
            ("note", store.list_notes()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.base.content.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
            ("prompt", store.list_prompts()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.template.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
            ("component", store.list_components()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.base.content.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
            ("link", store.list_links()?.into_iter().map(|e| (e.base.id.to_string(), e.base.title.clone(), e.base.content.clone(), e.base.tags.clone())).collect::<Vec<_>>()),
        ] {
            for (id, title, content, tags) in entities {
                match cache.compute_and_store_embedding_if_changed(
                    &id,
                    entity_type,
                    &title,
                    content.as_deref(),
                    &tags,
                    embedder,
                ) {
                    Ok(true) => embedding_count += 1,
                    Ok(false) => {} // Skipped (unchanged)
                    Err(_) => errors += 1,
                }
            }
        }
    }

    let stats = cache.get_stats()?;

    if json {
        #[derive(serde::Serialize)]
        struct RebuildResult {
            success: bool,
            entity_count: usize,
            embeddings_computed: usize,
            embedding_errors: usize,
        }

        let result = RebuildResult {
            success: true,
            entity_count: stats.entity_count,
            embeddings_computed: embedding_count,
            embedding_errors: errors,
        };

        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("Cache rebuilt successfully.");
        println!("  Entities indexed: {}", stats.entity_count);
        println!("  Embeddings computed: {}", embedding_count);
        if errors > 0 {
            eprintln!("  Embedding errors: {}", errors);
        }
    }

    Ok(())
}

// =============================================================================
// Snapshot handlers
// =============================================================================

/// Handle snapshot generation command.
pub fn handle_snapshot(output: Option<String>, verbose: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    let snapshot_dir = output
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(".medulla/snapshot"));

    let stats = crate::snapshot::generate_snapshot(&store, &snapshot_dir)?;

    if verbose {
        println!("Generated {} files:", stats.files_generated.len());
        for file in &stats.files_generated {
            println!("  {}", file);
        }
        println!();
    }

    println!(
        "Snapshot generated: {} decisions, {} tasks ({} active), {} notes, {} prompts, {} components, {} links",
        stats.decisions,
        stats.tasks_total,
        stats.tasks_active,
        stats.notes,
        stats.prompts,
        stats.components,
        stats.links,
    );

    println!("Output: {}", snapshot_dir.display());

    Ok(())
}

// =============================================================================
// Git hook handlers
// =============================================================================

/// Marker comment to identify Medulla hooks
const HOOK_MARKER: &str = "# MEDULLA_HOOK";

/// Pre-commit hook script template
const PRECOMMIT_HOOK: &str = r#"#!/bin/sh
# MEDULLA_HOOK - Auto-generated by medulla. Do not edit.
# This hook regenerates markdown snapshots when loro.db changes.

# Fast-path: skip if loro.db not staged
if ! git diff --cached --name-only | grep -q '\.medulla/loro\.db'; then
    exit 0
fi

# Generate snapshot
if ! medulla snapshot; then
    echo ""
    echo "Error: Medulla snapshot generation failed."
    echo "Fix the issue or use 'git commit --no-verify' to skip."
    exit 1
fi

# Stage generated snapshot files
git add .medulla/snapshot/

exit 0
"#;

/// Find the .git directory
fn find_git_dir(root: &std::path::Path) -> Option<PathBuf> {
    let git_dir = root.join(".git");
    if git_dir.is_dir() {
        Some(git_dir)
    } else if git_dir.is_file() {
        // Handle git worktrees - .git is a file pointing to the actual git dir
        if let Ok(content) = std::fs::read_to_string(&git_dir) {
            if let Some(path) = content.strip_prefix("gitdir: ") {
                let path = path.trim();
                let git_path = if std::path::Path::new(path).is_absolute() {
                    PathBuf::from(path)
                } else {
                    root.join(path)
                };
                if git_path.is_dir() {
                    return Some(git_path);
                }
            }
        }
        None
    } else {
        None
    }
}

/// Check if a hook file is a Medulla hook
fn is_medulla_hook(hook_path: &std::path::Path) -> bool {
    if let Ok(content) = std::fs::read_to_string(hook_path) {
        content.contains(HOOK_MARKER)
    } else {
        false
    }
}

/// Handle hook install command.
pub fn handle_hook_install(force: bool) -> Result<()> {
    let root = find_project_root();

    // Verify we're in a medulla project
    if !root.join(".medulla").exists() {
        return Err(MedullaError::NotInitialized);
    }

    let git_dir = find_git_dir(&root).ok_or_else(|| {
        MedullaError::Storage("Not a git repository. Run 'git init' first.".to_string())
    })?;

    let hooks_dir = git_dir.join("hooks");
    std::fs::create_dir_all(&hooks_dir)?;

    let hook_path = hooks_dir.join("pre-commit");

    // Check for existing hook
    if hook_path.exists() {
        if is_medulla_hook(&hook_path) {
            println!("Medulla hook already installed. Use --force to reinstall.");
            return Ok(());
        } else if !force {
            return Err(MedullaError::Storage(
                "A pre-commit hook already exists. Use --force to overwrite, or manually integrate medulla.".to_string()
            ));
        } else {
            // Backup existing hook
            let backup_path = hooks_dir.join("pre-commit.backup");
            std::fs::rename(&hook_path, &backup_path)?;
            println!("Backed up existing hook to pre-commit.backup");
        }
    }

    // Write hook
    std::fs::write(&hook_path, PRECOMMIT_HOOK)?;

    // Make executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&hook_path, perms)?;
    }

    println!("Installed pre-commit hook.");
    println!("Snapshots will be auto-generated when .medulla/loro.db is committed.");

    Ok(())
}

/// Handle hook uninstall command.
pub fn handle_hook_uninstall() -> Result<()> {
    let root = find_project_root();

    let git_dir = find_git_dir(&root).ok_or_else(|| {
        MedullaError::Storage("Not a git repository.".to_string())
    })?;

    let hook_path = git_dir.join("hooks/pre-commit");

    if !hook_path.exists() {
        println!("No pre-commit hook installed.");
        return Ok(());
    }

    if !is_medulla_hook(&hook_path) {
        return Err(MedullaError::Storage(
            "Pre-commit hook is not a Medulla hook. Remove it manually if needed.".to_string()
        ));
    }

    std::fs::remove_file(&hook_path)?;
    println!("Uninstalled Medulla pre-commit hook.");

    // Check for backup
    let backup_path = git_dir.join("hooks/pre-commit.backup");
    if backup_path.exists() {
        println!("Note: A backup of your previous hook exists at pre-commit.backup");
    }

    Ok(())
}

/// Handle hook status command.
pub fn handle_hook_status() -> Result<()> {
    let root = find_project_root();

    let git_dir = match find_git_dir(&root) {
        Some(dir) => dir,
        None => {
            println!("Status: Not a git repository");
            return Ok(());
        }
    };

    let hook_path = git_dir.join("hooks/pre-commit");

    if !hook_path.exists() {
        println!("Status: Not installed");
        println!("Run 'medulla hook install' to enable automatic snapshots.");
    } else if is_medulla_hook(&hook_path) {
        println!("Status: Installed");
        println!("Snapshots will be auto-generated when .medulla/loro.db is committed.");
    } else {
        println!("Status: Custom hook exists");
        println!("A non-Medulla pre-commit hook is installed.");
        println!("Use 'medulla hook install --force' to replace it.");
    }

    Ok(())
}
