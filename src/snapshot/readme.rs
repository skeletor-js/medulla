// src/snapshot/readme.rs
//! README index generation for snapshot

use std::path::Path;

use crate::entity::{Component, Decision, TaskStatus};
use crate::storage::LoroStore;
use crate::Result;

use super::SnapshotStats;
use super::utils::{format_date, slugify, write_snapshot_file};
use super::current_timestamp;

/// A recent activity entry for display
struct RecentActivity {
    entity_type: String,
    title: String,
    link: String,
    status: Option<String>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

/// Collect recent activity from all entity types
fn collect_recent_activity(store: &LoroStore) -> Result<Vec<RecentActivity>> {
    let mut activities = Vec::new();

    // Decisions
    for decision in store.list_decisions()? {
        let slug = slugify(&decision.base.title);
        let filename = format!("{:03}-{}.md", decision.base.sequence_number, slug);
        activities.push(RecentActivity {
            entity_type: "Decision".to_string(),
            title: decision.base.title.clone(),
            link: format!("decisions/{}", filename),
            status: Some(decision.status.to_string()),
            updated_at: decision.base.updated_at,
        });
    }

    // Tasks (only show active tasks in recent activity)
    for task in store.list_tasks()? {
        if task.status != TaskStatus::Done {
            activities.push(RecentActivity {
                entity_type: "Task".to_string(),
                title: task.base.title.clone(),
                link: format!("tasks/active.md#{}",  task.base.sequence_number),
                status: Some(task.status.to_string()),
                updated_at: task.base.updated_at,
            });
        }
    }

    // Notes
    for note in store.list_notes()? {
        let slug = slugify(&note.base.title);
        activities.push(RecentActivity {
            entity_type: "Note".to_string(),
            title: note.base.title.clone(),
            link: format!("notes/{}.md", slug),
            status: note.note_type.clone(),
            updated_at: note.base.updated_at,
        });
    }

    // Prompts
    for prompt in store.list_prompts()? {
        let slug = slugify(&prompt.base.title);
        activities.push(RecentActivity {
            entity_type: "Prompt".to_string(),
            title: prompt.base.title.clone(),
            link: format!("prompts/{}.md", slug),
            status: None,
            updated_at: prompt.base.updated_at,
        });
    }

    // Components
    for component in store.list_components()? {
        let slug = slugify(&component.base.title);
        activities.push(RecentActivity {
            entity_type: "Component".to_string(),
            title: component.base.title.clone(),
            link: format!("components/{}.md", slug),
            status: Some(component.status.to_string()),
            updated_at: component.base.updated_at,
        });
    }

    // Links
    for link in store.list_links()? {
        let slug = slugify(&link.base.title);
        activities.push(RecentActivity {
            entity_type: "Link".to_string(),
            title: link.base.title.clone(),
            link: format!("links/{}.md", slug),
            status: link.link_type.clone(),
            updated_at: link.base.updated_at,
        });
    }

    // Sort by updated_at descending
    activities.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(activities)
}

/// Generate decisions quick links section
fn generate_decisions_section(decisions: &[Decision]) -> String {
    if decisions.is_empty() {
        return String::new();
    }

    let mut section = String::from("### Decisions\n\n");

    let mut sorted = decisions.to_vec();
    sorted.sort_by_key(|d| d.base.sequence_number);

    for decision in &sorted {
        let slug = slugify(&decision.base.title);
        let filename = format!("{:03}-{}.md", decision.base.sequence_number, slug);
        section.push_str(&format!(
            "- [{:03} - {}](decisions/{}) `{}`\n",
            decision.base.sequence_number,
            decision.base.title,
            filename,
            decision.status
        ));
    }

    section.push('\n');
    section
}

/// Generate components quick links section
fn generate_components_section(components: &[Component]) -> String {
    if components.is_empty() {
        return String::new();
    }

    let mut section = String::from("### Components\n\n");

    let mut sorted = components.to_vec();
    sorted.sort_by_key(|c| c.base.sequence_number);

    for component in &sorted {
        let slug = slugify(&component.base.title);
        section.push_str(&format!(
            "- [{}](components/{}.md) `{}`\n",
            component.base.title,
            slug,
            component.status
        ));
    }

    section.push('\n');
    section
}

/// Generate README.md index
pub fn generate(store: &LoroStore, snapshot_dir: &Path, stats: &SnapshotStats) -> Result<()> {
    let mut content = String::from("# Project Knowledge Base\n\n");
    content.push_str("> Auto-generated by [Medulla](https://github.com/jordanstella/medulla). Do not edit directly.\n\n");

    // Summary table
    content.push_str("## Summary\n\n");
    content.push_str("| Type | Count |\n");
    content.push_str("|------|-------|\n");
    content.push_str(&format!("| Decisions | {} |\n", stats.decisions));
    content.push_str(&format!("| Tasks | {} ({} active) |\n", stats.tasks_total, stats.tasks_active));
    content.push_str(&format!("| Notes | {} |\n", stats.notes));
    content.push_str(&format!("| Prompts | {} |\n", stats.prompts));
    content.push_str(&format!("| Components | {} |\n", stats.components));
    content.push_str(&format!("| Links | {} |\n", stats.links));
    content.push('\n');

    // Check if we have any entities
    if stats.total_entities() == 0 {
        content.push_str("*No entities yet. Use `medulla add` to create your first entity.*\n\n");
    } else {
        // Recent Activity (top 5)
        let activities = collect_recent_activity(store)?;
        if !activities.is_empty() {
            content.push_str("## Recent Activity\n\n");
            for activity in activities.iter().take(5) {
                let status_str = activity.status.as_ref()
                    .map(|s| format!(" - {}", s))
                    .unwrap_or_default();
                content.push_str(&format!(
                    "- **{}**: [{}]({}){}  \n  _{}_\n",
                    activity.entity_type,
                    activity.title,
                    activity.link,
                    status_str,
                    format_date(&activity.updated_at),
                ));
            }
            content.push('\n');
        }

        // Quick Links
        content.push_str("## Quick Links\n\n");

        // Decisions
        let decisions = store.list_decisions()?;
        content.push_str(&generate_decisions_section(&decisions));

        // Active Tasks
        if stats.tasks_active > 0 {
            content.push_str("### Active Tasks\n\n");
            content.push_str("See [tasks/active.md](tasks/active.md)\n\n");
        }

        // Components
        let components = store.list_components()?;
        content.push_str(&generate_components_section(&components));

        // Notes
        if stats.notes > 0 {
            content.push_str("### Notes\n\n");
            let notes = store.list_notes()?;
            let mut sorted = notes;
            sorted.sort_by(|a, b| b.base.updated_at.cmp(&a.base.updated_at));
            for note in sorted.iter().take(5) {
                let slug = slugify(&note.base.title);
                let type_str = note.note_type.as_ref()
                    .map(|t| format!(" `{}`", t))
                    .unwrap_or_default();
                content.push_str(&format!(
                    "- [{}](notes/{}.md){}\n",
                    note.base.title,
                    slug,
                    type_str,
                ));
            }
            if notes.len() > 5 {
                content.push_str(&format!("\n*...and {} more notes*\n", notes.len() - 5));
            }
            content.push('\n');
        }

        // Prompts
        if stats.prompts > 0 {
            content.push_str("### Prompts\n\n");
            let prompts = store.list_prompts()?;
            let mut sorted = prompts;
            sorted.sort_by_key(|p| p.base.sequence_number);
            for prompt in &sorted {
                let slug = slugify(&prompt.base.title);
                content.push_str(&format!(
                    "- [{}](prompts/{}.md)\n",
                    prompt.base.title,
                    slug,
                ));
            }
            content.push('\n');
        }

        // Links
        if stats.links > 0 {
            content.push_str("### Links\n\n");
            let links = store.list_links()?;
            let mut sorted = links;
            sorted.sort_by_key(|l| l.base.sequence_number);
            for link in &sorted {
                let slug = slugify(&link.base.title);
                let type_str = link.link_type.as_ref()
                    .map(|t| format!(" `{}`", t))
                    .unwrap_or_default();
                content.push_str(&format!(
                    "- [{}](links/{}.md){}\n",
                    link.base.title,
                    slug,
                    type_str,
                ));
            }
            content.push('\n');
        }
    }

    // Footer
    content.push_str("---\n\n");
    content.push_str(&format!("*Generated: {}*\n", current_timestamp()));

    let readme_path = snapshot_dir.join("README.md");
    write_snapshot_file(&readme_path, &content)?;

    Ok(())
}
