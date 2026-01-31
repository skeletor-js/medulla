use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::cache::SqliteCache;
use crate::entity::{
    Component, Decision, DecisionStatus, Link, Note, Prompt, Relation, RelationType, Task,
};
use crate::error::{MedullaError, Result};
use crate::storage::{DecisionUpdate, LoroStore};

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

    let target_id = uuid::Uuid::parse_str(parts[1]).map_err(|_| {
        MedullaError::Storage(format!("Invalid UUID in relation: {}", parts[1]))
    })?;

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

    // Find the decision by ID
    let decisions = store.list_decisions()?;
    let decision = if let Ok(seq) = id.parse::<u32>() {
        decisions.iter().find(|d| d.base.sequence_number == seq)
    } else {
        decisions
            .iter()
            .find(|d| d.base.id.to_string().starts_with(&id))
    };

    let decision = match decision {
        Some(d) => d.clone(),
        None => return Err(MedullaError::EntityNotFound(id)),
    };

    // Build update payload
    let mut updates = DecisionUpdate::default();
    updates.title = title;
    updates.status = status.and_then(|s| s.parse::<DecisionStatus>().ok());
    updates.add_tags = tags;
    updates.remove_tags = remove_tags;

    // Read content from stdin if requested
    if stdin {
        let mut content = String::new();
        io::stdin().read_to_string(&mut content)?;
        if !content.is_empty() {
            updates.content = Some(content);
        }
    }

    // TODO: Handle --edit flag (Phase 4)
    if edit {
        eprintln!("Warning: --edit flag not yet implemented, skipping");
    }

    // Apply updates
    store.update_decision(&decision.base.id, updates)?;

    // Handle new relations
    let git_author = get_git_author();
    for rel_str in &relations {
        match parse_relation_string(rel_str) {
            Ok((rel_type, target_id)) => {
                let mut relation = Relation::new(
                    decision.base.id,
                    "decision".to_string(),
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

    store.save()?;

    // Get the updated decision
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

    Ok(())
}

pub fn handle_delete(id: String, force: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;

    // Find the decision by ID
    let decisions = store.list_decisions()?;
    let decision = if let Ok(seq) = id.parse::<u32>() {
        decisions.iter().find(|d| d.base.sequence_number == seq)
    } else {
        decisions
            .iter()
            .find(|d| d.base.id.to_string().starts_with(&id))
    };

    let decision = match decision {
        Some(d) => d.clone(),
        None => return Err(MedullaError::EntityNotFound(id)),
    };

    // Confirm deletion unless --force is used
    if !force {
        eprintln!(
            "Delete decision {:03} ({}) - {}? [y/N] ",
            decision.base.sequence_number,
            &decision.base.id.to_string()[..7],
            decision.base.title
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
        if rel.source_id == decision.base.id || rel.target_id == decision.base.id {
            let _ = store.delete_relation(
                &rel.source_id.to_string(),
                &rel.relation_type.to_string(),
                &rel.target_id.to_string(),
            );
        }
    }

    // Delete the decision
    store.delete_decision(&decision.base.id)?;
    store.save()?;

    println!(
        "Deleted decision {:03} ({}) - {}",
        decision.base.sequence_number,
        &decision.base.id.to_string()[..7],
        decision.base.title
    );

    Ok(())
}

pub fn handle_search(query: String, json: bool) -> Result<()> {
    let root = find_project_root();
    let store = LoroStore::open(&root)?;
    let cache = SqliteCache::open(store.medulla_dir())?;

    // Sync cache with store
    store.sync_cache(&cache)?;

    // Perform search
    let results = cache.search_decisions(&query)?;

    if json {
        #[derive(serde::Serialize)]
        struct SearchResultJson {
            id: String,
            sequence_number: u32,
            title: String,
            status: String,
            snippet: Option<String>,
        }

        let json_results: Vec<SearchResultJson> = results
            .into_iter()
            .map(|r| SearchResultJson {
                id: r.id,
                sequence_number: r.sequence_number,
                title: r.title,
                status: r.status,
                snippet: r.content_snippet,
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else if results.is_empty() {
        println!("No results found for '{}'.", query);
    } else {
        println!("Search results for '{}':\n", query);
        for r in results {
            println!(
                "  {:03} ({}) [{}] {}",
                r.sequence_number,
                &r.id[..7.min(r.id.len())],
                r.status,
                r.title
            );
            if let Some(snippet) = r.content_snippet {
                // Clean up FTS5 snippet
                let clean_snippet = snippet
                    .replace("<mark>", "\x1b[1m")
                    .replace("</mark>", "\x1b[0m");
                println!("      {}", clean_snippet);
            }
        }
    }

    Ok(())
}
