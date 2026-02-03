// src/snapshot/task.rs
//! Task snapshot generation

use std::path::Path;

use crate::entity::{Task, TaskPriority, TaskStatus};
use crate::storage::LoroStore;
use crate::Result;

use super::GeneratedFile;
use super::utils::{short_uuid, write_snapshot_file};
use super::current_timestamp;

/// Format a single task line
fn format_task_line(task: &Task) -> String {
    let checkbox = match task.status {
        TaskStatus::Done => "[x]",
        _ => "[ ]",
    };

    let blocked_indicator = if task.status == TaskStatus::Blocked {
        " `[blocked]`"
    } else {
        ""
    };

    let mut line = format!(
        "- {} **{}** `#{}` `({})`{}",
        checkbox,
        task.base.title,
        task.base.sequence_number,
        short_uuid(&task.base.id),
        blocked_indicator,
    );

    // Build metadata line
    let mut meta_parts = Vec::new();

    if let Some(due) = task.due_date {
        meta_parts.push(format!("Due: {}", due));
    }

    if let Some(assignee) = &task.assignee {
        meta_parts.push(format!("Assignee: {}", assignee));
    }

    if !task.base.tags.is_empty() {
        meta_parts.push(format!("Tags: {}", task.base.tags.join(", ")));
    }

    if !meta_parts.is_empty() {
        line.push_str(&format!("\n  {}", meta_parts.join(" | ")));
    }

    line
}

/// Format a completed task line (includes completion date approximation)
fn format_completed_task_line(task: &Task) -> String {
    let mut line = format!(
        "- [x] **{}** `#{}` `({})` - Completed {}",
        task.base.title,
        task.base.sequence_number,
        short_uuid(&task.base.id),
        task.base.updated_at.format("%Y-%m-%d"),
    );

    if !task.base.tags.is_empty() {
        line.push_str(&format!("\n  Tags: {}", task.base.tags.join(", ")));
    }

    line
}

/// Generate active.md with tasks grouped by priority
fn generate_active(tasks: &[&Task], snapshot_dir: &Path) -> Result<GeneratedFile> {
    let mut content = String::from("# Active Tasks\n\n");
    content.push_str("> Generated from Medulla. Do not edit directly.\n\n");

    if tasks.is_empty() {
        content.push_str("*No active tasks.*\n");
    } else {
        // Group by priority
        let priorities = [
            (TaskPriority::Urgent, "Urgent"),
            (TaskPriority::High, "High Priority"),
            (TaskPriority::Normal, "Normal Priority"),
            (TaskPriority::Low, "Low Priority"),
        ];

        for (priority, heading) in priorities {
            let priority_tasks: Vec<_> = tasks
                .iter()
                .filter(|t| t.priority == priority)
                .collect();

            if !priority_tasks.is_empty() {
                content.push_str(&format!("## {}\n\n", heading));

                // Sort by due date (soonest first), then by sequence number
                let mut sorted = priority_tasks;
                sorted.sort_by(|a, b| {
                    match (&a.due_date, &b.due_date) {
                        (Some(a_date), Some(b_date)) => a_date.cmp(b_date),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.base.sequence_number.cmp(&b.base.sequence_number),
                    }
                });

                for task in sorted {
                    content.push_str(&format_task_line(task));
                    content.push_str("\n\n");
                }
            }
        }
    }

    content.push_str("---\n\n");
    content.push_str(&format!("*Last updated: {}*\n", current_timestamp()));

    let file_path = snapshot_dir.join("tasks/active.md");
    write_snapshot_file(&file_path, &content)?;

    Ok(GeneratedFile {
        relative_path: "tasks/active.md".to_string(),
        entity_count: tasks.len(),
    })
}

/// Generate completed.md with done tasks
fn generate_completed(tasks: &[&Task], snapshot_dir: &Path) -> Result<GeneratedFile> {
    let mut content = String::from("# Completed Tasks\n\n");
    content.push_str("> Generated from Medulla. Do not edit directly.\n\n");

    if tasks.is_empty() {
        content.push_str("*No completed tasks.*\n");
    } else {
        // Sort by updated_at (most recent first)
        let mut sorted: Vec<_> = tasks.iter().copied().collect();
        sorted.sort_by(|a, b| b.base.updated_at.cmp(&a.base.updated_at));

        for task in sorted {
            content.push_str(&format_completed_task_line(task));
            content.push_str("\n\n");
        }
    }

    content.push_str("---\n\n");
    content.push_str(&format!("*Last updated: {}*\n", current_timestamp()));

    let file_path = snapshot_dir.join("tasks/completed.md");
    write_snapshot_file(&file_path, &content)?;

    Ok(GeneratedFile {
        relative_path: "tasks/completed.md".to_string(),
        entity_count: tasks.len(),
    })
}

/// Generate task snapshot files
pub fn generate(store: &LoroStore, snapshot_dir: &Path) -> Result<Vec<GeneratedFile>> {
    let tasks = store.list_tasks()?;
    let mut generated = Vec::new();

    // Split into active and completed
    let active: Vec<_> = tasks
        .iter()
        .filter(|t| t.status != TaskStatus::Done)
        .collect();

    let completed: Vec<_> = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .collect();

    generated.push(generate_active(&active, snapshot_dir)?);
    generated.push(generate_completed(&completed, snapshot_dir)?);

    Ok(generated)
}
