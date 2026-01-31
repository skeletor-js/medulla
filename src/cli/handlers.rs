use std::env;
use std::io::{self, Read};
use std::path::PathBuf;

use crate::entity::Decision;
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
            eprintln!(
                "Entity type '{}' not yet supported. Try 'decision'.",
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
