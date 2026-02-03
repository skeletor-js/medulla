// src/snapshot/task.rs
//! Task snapshot generation

use std::path::Path;

use crate::entity::{Task, TaskPriority, TaskStatus};
use crate::storage::LoroStore;
use crate::Result;

use super::current_timestamp;
use super::utils::{short_uuid, write_snapshot_file};
use super::GeneratedFile;

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
            let priority_tasks: Vec<_> = tasks.iter().filter(|t| t.priority == priority).collect();

            if !priority_tasks.is_empty() {
                content.push_str(&format!("## {}\n\n", heading));

                // Sort by due date (soonest first), then by sequence number
                let mut sorted = priority_tasks;
                sorted.sort_by(|a, b| match (&a.due_date, &b.due_date) {
                    (Some(a_date), Some(b_date)) => a_date.cmp(b_date),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.base.sequence_number.cmp(&b.base.sequence_number),
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
        let mut sorted: Vec<_> = tasks.to_vec();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityBase;
    use chrono::{NaiveDate, Utc};
    use tempfile::TempDir;

    fn mock_task(seq: u32, title: &str, status: TaskStatus, priority: TaskPriority) -> Task {
        let base = EntityBase::new(title.to_string(), seq);
        Task {
            base,
            status,
            priority,
            due_date: None,
            assignee: None,
        }
    }

    #[test]
    fn test_format_task_line_incomplete() {
        let task = mock_task(1, "Test Task", TaskStatus::Todo, TaskPriority::Normal);
        let line = format_task_line(&task);

        assert!(line.contains("[ ]"));
        assert!(line.contains("**Test Task**"));
        assert!(line.contains("#1"));
    }

    #[test]
    fn test_format_task_line_completed() {
        let task = mock_task(1, "Done Task", TaskStatus::Done, TaskPriority::Normal);
        let line = format_task_line(&task);

        assert!(line.contains("[x]"));
    }

    #[test]
    fn test_format_task_line_blocked_indicator() {
        let task = mock_task(1, "Blocked Task", TaskStatus::Blocked, TaskPriority::Normal);
        let line = format_task_line(&task);

        assert!(line.contains("`[blocked]`"));
    }

    #[test]
    fn test_format_task_line_with_due_date() {
        let mut task = mock_task(1, "Task", TaskStatus::Todo, TaskPriority::Normal);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        let line = format_task_line(&task);

        assert!(line.contains("Due: 2025-06-15"));
    }

    #[test]
    fn test_format_task_line_with_assignee() {
        let mut task = mock_task(1, "Task", TaskStatus::Todo, TaskPriority::Normal);
        task.assignee = Some("Alice".to_string());
        let line = format_task_line(&task);

        assert!(line.contains("Assignee: Alice"));
    }

    #[test]
    fn test_format_task_line_with_tags() {
        let mut task = mock_task(1, "Task", TaskStatus::Todo, TaskPriority::Normal);
        task.base.tags = vec!["urgent".to_string(), "backend".to_string()];
        let line = format_task_line(&task);

        assert!(line.contains("Tags: urgent, backend"));
    }

    #[test]
    fn test_format_completed_task_line() {
        let task = mock_task(1, "Completed Task", TaskStatus::Done, TaskPriority::Normal);
        let line = format_completed_task_line(&task);

        assert!(line.contains("[x]"));
        assert!(line.contains("**Completed Task**"));
        assert!(line.contains("Completed"));
    }

    #[test]
    fn test_generate_active_empty() {
        let tmp = TempDir::new().unwrap();
        let snapshot_dir = tmp.path();
        std::fs::create_dir_all(snapshot_dir.join("tasks")).unwrap();

        let tasks: Vec<&Task> = vec![];
        let result = generate_active(&tasks, snapshot_dir).unwrap();

        assert_eq!(result.relative_path, "tasks/active.md");
        assert_eq!(result.entity_count, 0);

        let content = std::fs::read_to_string(snapshot_dir.join("tasks/active.md")).unwrap();
        assert!(content.contains("*No active tasks.*"));
    }

    #[test]
    fn test_generate_active_grouped_by_priority() {
        let tmp = TempDir::new().unwrap();
        let snapshot_dir = tmp.path();
        std::fs::create_dir_all(snapshot_dir.join("tasks")).unwrap();

        let urgent = mock_task(1, "Urgent Task", TaskStatus::Todo, TaskPriority::Urgent);
        let high = mock_task(2, "High Task", TaskStatus::Todo, TaskPriority::High);
        let normal = mock_task(3, "Normal Task", TaskStatus::Todo, TaskPriority::Normal);
        let low = mock_task(4, "Low Task", TaskStatus::Todo, TaskPriority::Low);

        let tasks: Vec<&Task> = vec![&low, &normal, &urgent, &high]; // Intentionally out of order
        generate_active(&tasks, snapshot_dir).unwrap();

        let content = std::fs::read_to_string(snapshot_dir.join("tasks/active.md")).unwrap();

        // Check priority headings are present
        assert!(content.contains("## Urgent"));
        assert!(content.contains("## High Priority"));
        assert!(content.contains("## Normal Priority"));
        assert!(content.contains("## Low Priority"));

        // Check order: Urgent should appear before Low
        let urgent_pos = content.find("Urgent Task").unwrap();
        let high_pos = content.find("High Task").unwrap();
        let normal_pos = content.find("Normal Task").unwrap();
        let low_pos = content.find("Low Task").unwrap();

        assert!(urgent_pos < high_pos);
        assert!(high_pos < normal_pos);
        assert!(normal_pos < low_pos);
    }

    #[test]
    fn test_generate_active_sorted_by_due_date() {
        let tmp = TempDir::new().unwrap();
        let snapshot_dir = tmp.path();
        std::fs::create_dir_all(snapshot_dir.join("tasks")).unwrap();

        let mut task1 = mock_task(1, "Later Task", TaskStatus::Todo, TaskPriority::Normal);
        task1.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());

        let mut task2 = mock_task(2, "Sooner Task", TaskStatus::Todo, TaskPriority::Normal);
        task2.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());

        let tasks: Vec<&Task> = vec![&task1, &task2]; // Later task first
        generate_active(&tasks, snapshot_dir).unwrap();

        let content = std::fs::read_to_string(snapshot_dir.join("tasks/active.md")).unwrap();

        // Sooner task should appear before later task
        let sooner_pos = content.find("Sooner Task").unwrap();
        let later_pos = content.find("Later Task").unwrap();
        assert!(sooner_pos < later_pos);
    }

    #[test]
    fn test_generate_completed_empty() {
        let tmp = TempDir::new().unwrap();
        let snapshot_dir = tmp.path();
        std::fs::create_dir_all(snapshot_dir.join("tasks")).unwrap();

        let tasks: Vec<&Task> = vec![];
        let result = generate_completed(&tasks, snapshot_dir).unwrap();

        assert_eq!(result.relative_path, "tasks/completed.md");
        assert_eq!(result.entity_count, 0);

        let content = std::fs::read_to_string(snapshot_dir.join("tasks/completed.md")).unwrap();
        assert!(content.contains("*No completed tasks.*"));
    }

    #[test]
    fn test_generate_completed_sorted_by_date() {
        let tmp = TempDir::new().unwrap();
        let snapshot_dir = tmp.path();
        std::fs::create_dir_all(snapshot_dir.join("tasks")).unwrap();

        let mut task1 = mock_task(1, "Old Task", TaskStatus::Done, TaskPriority::Normal);
        task1.base.updated_at = Utc::now() - chrono::Duration::days(7);

        let mut task2 = mock_task(2, "Recent Task", TaskStatus::Done, TaskPriority::Normal);
        task2.base.updated_at = Utc::now();

        let tasks: Vec<&Task> = vec![&task1, &task2]; // Old first
        generate_completed(&tasks, snapshot_dir).unwrap();

        let content = std::fs::read_to_string(snapshot_dir.join("tasks/completed.md")).unwrap();

        // Recent should appear before old (most recent first)
        let recent_pos = content.find("Recent Task").unwrap();
        let old_pos = content.find("Old Task").unwrap();
        assert!(recent_pos < old_pos);
    }

    #[test]
    fn test_generate_splits_active_and_completed() {
        let tmp = TempDir::new().unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        std::fs::create_dir_all(&medulla_dir).unwrap();

        let store = crate::storage::LoroStore::init(&medulla_dir).unwrap();

        // Add an active task
        let active_task = Task::new("Active Task".to_string(), 1);
        store.add_task(&active_task).unwrap();

        // Add a completed task
        let mut completed_task = Task::new("Completed Task".to_string(), 2);
        completed_task.status = TaskStatus::Done;
        store.add_task(&completed_task).unwrap();

        let snapshot_dir = medulla_dir.join("snapshot");
        super::super::utils::ensure_snapshot_dirs(&snapshot_dir).unwrap();

        let files = generate(&store, &snapshot_dir).unwrap();

        assert_eq!(files.len(), 2);

        let active_content = std::fs::read_to_string(snapshot_dir.join("tasks/active.md")).unwrap();
        let completed_content =
            std::fs::read_to_string(snapshot_dir.join("tasks/completed.md")).unwrap();

        assert!(active_content.contains("Active Task"));
        assert!(!active_content.contains("Completed Task"));
        assert!(completed_content.contains("Completed Task"));
        assert!(!completed_content.contains("Active Task"));
    }
}
