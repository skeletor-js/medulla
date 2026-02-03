use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Duration;

use crate::cache::SqliteCache;
use crate::entity::{
    Component, ComponentStatus, Decision, DecisionStatus, Link, Note, Prompt, Relation,
    RelationType, Task, TaskStatus,
};
use crate::error::{MedullaError, Result};
use crate::mcp::MedullaServer;
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

pub fn handle_search(query: String, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    // Perform search across all entity types
    let results = cache.search_all(&query, 50)?;

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
