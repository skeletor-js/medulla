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
        .args([
            "add",
            "decision",
            "Use Rust",
            "--status=accepted",
            "--tag=lang",
        ])
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

#[test]
fn test_update_decision() {
    let tmp = TempDir::new().unwrap();

    // Init and add a decision
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Original Title",
            "--status=proposed",
            "--tag=original",
        ])
        .output()
        .unwrap();

    // Update the decision
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "update",
            "1",
            "--title=Updated Title",
            "--status=accepted",
            "--tag=new-tag",
            "--remove-tag=original",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated"));
    assert!(stdout.contains("Updated Title"));

    // Verify with get
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["get", "1", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    // Note: EntityBase fields are flattened via #[serde(flatten)]
    assert_eq!(parsed["title"], "Updated Title");
    assert_eq!(parsed["status"], "accepted");
    let tags = parsed["tags"].as_array().unwrap();
    assert!(tags.contains(&serde_json::json!("new-tag")));
    assert!(!tags.contains(&serde_json::json!("original")));
}

#[test]
fn test_delete_decision_with_force() {
    let tmp = TempDir::new().unwrap();

    // Init and add a decision
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "To Be Deleted"])
        .output()
        .unwrap();

    // Verify it exists
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("To Be Deleted"));

    // Delete with --force
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["delete", "1", "--force"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deleted"));

    // Verify it's gone
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No decisions found"));
}

#[test]
fn test_delete_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["delete", "999", "--force"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Entity not found"));
}

#[test]
fn test_add_decision_with_relation() {
    let tmp = TempDir::new().unwrap();

    // Init
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add first decision and capture its UUID
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "First Decision", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    // Note: EntityBase fields are flattened via #[serde(flatten)]
    let first_id = parsed["id"].as_str().unwrap();

    // Add second decision with relation to first
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Second Decision",
            &format!("--relation=supersedes:{}", first_id),
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("002"));
}

#[test]
fn test_search_decisions() {
    let tmp = TempDir::new().unwrap();

    // Init
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add decisions with searchable content
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Use PostgreSQL for database",
            "--status=accepted",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Use Redis for caching",
            "--status=proposed",
        ])
        .output()
        .unwrap();

    // Search for PostgreSQL
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "PostgreSQL"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PostgreSQL"));
    assert!(!stdout.contains("Redis")); // Should not match

    // Search for database (should also find PostgreSQL decision)
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "database"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PostgreSQL"));

    // Search with JSON output
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "caching", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "Use Redis for caching");
}

#[test]
fn test_search_no_results() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Some decision"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "nonexistent"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No results found"));
}

// ============================================================================
// Task Queue CLI Tests (Batch 7)
// ============================================================================

#[test]
fn test_tasks_ready() {
    let tmp = TempDir::new().unwrap();

    // Init
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add tasks with different priorities
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "task",
            "High Priority Task",
            "--priority=high",
            "--status=todo",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "task",
            "Low Priority Task",
            "--priority=low",
            "--status=todo",
        ])
        .output()
        .unwrap();

    // Run tasks ready
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "ready"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Ready tasks"));
    assert!(stdout.contains("High Priority Task"));
    assert!(stdout.contains("Low Priority Task"));
    // High priority should appear before low
    let high_pos = stdout.find("High Priority Task").unwrap();
    let low_pos = stdout.find("Low Priority Task").unwrap();
    assert!(high_pos < low_pos);
}

#[test]
fn test_tasks_ready_json() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Test Task", "--status=todo"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "ready", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[test]
fn test_tasks_ready_empty() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "ready"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No ready tasks found"));
}

#[test]
fn test_tasks_next() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add tasks - urgent should come first
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "task",
            "Normal Task",
            "--priority=normal",
            "--status=todo",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "task",
            "Urgent Task",
            "--priority=urgent",
            "--status=todo",
        ])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "next"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Next task"));
    assert!(stdout.contains("Urgent Task"));
}

#[test]
fn test_tasks_next_json() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Only Task", "--status=todo"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "next", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["title"], "Only Task");
}

#[test]
fn test_tasks_blocked_empty() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add a task without blockers
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Free Task", "--status=todo"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "blocked"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No blocked tasks found"));
}

#[test]
fn test_tasks_ready_with_limit() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add multiple tasks
    for i in 1..=5 {
        medulla_cmd()
            .current_dir(tmp.path())
            .args(["add", "task", &format!("Task {}", i), "--status=todo"])
            .output()
            .unwrap();
    }

    // Limit to 2
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["tasks", "ready", "--limit=2", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn test_serve_without_init() {
    let tmp = TempDir::new().unwrap();

    // Try to start server without initializing
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["serve"])
        .output()
        .unwrap();

    // Should fail because not initialized
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not a medulla project") || stderr.contains("init"));
}

// ============================================================================
// Multi-entity type tests
// ============================================================================

#[test]
fn test_add_and_list_all_entity_types() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add one of each entity type
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Test Decision"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Test Task", "--status=todo"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "note", "Test Note"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "prompt", "Test Prompt"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "component", "Test Component"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "link", "Test Link", "--url=https://example.com"])
        .output()
        .unwrap();

    // List each type
    for entity_type in [
        "decisions",
        "tasks",
        "notes",
        "prompts",
        "components",
        "links",
    ] {
        let output = medulla_cmd()
            .current_dir(tmp.path())
            .args(["list", entity_type])
            .output()
            .unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Test") || stdout.contains("001"));
    }
}

#[test]
fn test_search_across_entity_types() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add entities with "database" in title
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Use database X"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Set up database", "--status=todo"])
        .output()
        .unwrap();

    // Search should find both
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "database", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

// ============================================================================
// Snapshot CLI Integration Tests (Phase 4)
// ============================================================================

#[test]
fn test_snapshot_command_basic() {
    let tmp = TempDir::new().unwrap();

    // Init
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add a decision so there's something to snapshot
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Test Decision", "--status=accepted"])
        .output()
        .unwrap();

    // Run snapshot
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Snapshot generated"));

    // Verify files were created
    let snapshot_dir = tmp.path().join(".medulla/snapshot");
    assert!(snapshot_dir.exists());
    assert!(snapshot_dir.join("README.md").exists());
    assert!(snapshot_dir.join("decisions").exists());
}

#[test]
fn test_snapshot_command_verbose() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Verbose Test", "--status=accepted"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot", "--verbose"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Generated"));
    assert!(stdout.contains("README.md"));
    assert!(stdout.contains("decisions/"));
}

#[test]
fn test_snapshot_with_all_entity_types() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add one of each entity type
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Architecture Choice",
            "--status=accepted",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Implement feature", "--status=todo"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "note", "Meeting notes", "--type=meeting"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "prompt", "Code review prompt"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "component", "Auth service", "--status=active"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "link",
            "Documentation",
            "--url=https://docs.example.com",
        ])
        .output()
        .unwrap();

    // Generate snapshot
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1 decisions"));
    assert!(stdout.contains("1 tasks"));
    assert!(stdout.contains("1 notes"));
    assert!(stdout.contains("1 prompts"));
    assert!(stdout.contains("1 components"));
    assert!(stdout.contains("1 links"));

    // Verify subdirectories
    let snapshot_dir = tmp.path().join(".medulla/snapshot");
    assert!(snapshot_dir.join("decisions").exists());
    assert!(snapshot_dir.join("tasks").exists());
    assert!(snapshot_dir.join("notes").exists());
    assert!(snapshot_dir.join("prompts").exists());
    assert!(snapshot_dir.join("components").exists());
    assert!(snapshot_dir.join("links").exists());
}

#[test]
fn test_snapshot_readme_content() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Use PostgreSQL", "--status=accepted"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    let readme_path = tmp.path().join(".medulla/snapshot/README.md");
    let readme_content = std::fs::read_to_string(&readme_path).unwrap();

    // Verify README structure
    assert!(readme_content.contains("# Project Knowledge Base"));
    assert!(readme_content.contains("## Summary"));
    assert!(readme_content.contains("| Decisions | 1 |"));
    assert!(readme_content.contains("decisions/"));
    assert!(readme_content.contains("Use PostgreSQL"));
    assert!(readme_content.contains("*Generated:"));
}

#[test]
fn test_snapshot_decision_file_content() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Use Rust",
            "--status=accepted",
            "--tag=language",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    // Find the decision file
    let decisions_dir = tmp.path().join(".medulla/snapshot/decisions");
    let entries: Vec<_> = std::fs::read_dir(&decisions_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    assert_eq!(entries.len(), 1);
    let file_path = entries[0].path();
    let content = std::fs::read_to_string(&file_path).unwrap();

    // Verify YAML frontmatter
    assert!(content.starts_with("---\n"));
    assert!(content.contains("title: Use Rust"));
    assert!(content.contains("status: accepted"));
    assert!(content.contains("sequence: 1"));
    assert!(content.contains("tags:"));
    assert!(content.contains("- language"));
}

#[test]
fn test_snapshot_task_files() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add active task
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "task",
            "Active Task",
            "--status=todo",
            "--priority=high",
        ])
        .output()
        .unwrap();

    // Add completed task
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Done Task", "--status=done"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    let tasks_dir = tmp.path().join(".medulla/snapshot/tasks");

    // Verify active.md exists and has content
    let active_path = tasks_dir.join("active.md");
    assert!(active_path.exists());
    let active_content = std::fs::read_to_string(&active_path).unwrap();
    assert!(active_content.contains("Active Task"));
    assert!(active_content.contains("[ ]")); // Checkbox format

    // Verify completed.md exists and has content
    let completed_path = tasks_dir.join("completed.md");
    assert!(completed_path.exists());
    let completed_content = std::fs::read_to_string(&completed_path).unwrap();
    assert!(completed_content.contains("Done Task"));
    assert!(completed_content.contains("[x]")); // Completed checkbox
}

#[test]
fn test_snapshot_without_init_fails() {
    let tmp = TempDir::new().unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not in a medulla project") || stderr.contains("init"));
}

#[test]
fn test_snapshot_empty_store() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["snapshot"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0 decisions"));
    assert!(stdout.contains("0 tasks"));

    // README should still be generated
    let readme_path = tmp.path().join(".medulla/snapshot/README.md");
    assert!(readme_path.exists());
    let readme = std::fs::read_to_string(&readme_path).unwrap();
    assert!(readme.contains("No entities yet"));
}

// ============================================================================
// Hook CLI Integration Tests (Phase 4)
// ============================================================================

#[test]
fn test_hook_install_command() {
    let tmp = TempDir::new().unwrap();

    // Initialize git repo first
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Initialize medulla
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Install hook
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installed"));

    // Verify hook file exists
    let hook_path = tmp.path().join(".git/hooks/pre-commit");
    assert!(hook_path.exists());
}

#[test]
fn test_hook_status_not_installed() {
    let tmp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Initialize medulla
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Check status without installing
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "status"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Not installed"));
}

#[test]
fn test_hook_status_installed() {
    let tmp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Initialize medulla
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Install hook
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install"])
        .output()
        .unwrap();

    // Check status
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "status"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Installed"));
}

#[test]
fn test_hook_uninstall_command() {
    let tmp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Initialize medulla
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Install hook first
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install"])
        .output()
        .unwrap();

    let hook_path = tmp.path().join(".git/hooks/pre-commit");
    assert!(hook_path.exists());

    // Uninstall hook
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "uninstall"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Uninstalled"));

    // Verify hook is removed
    assert!(!hook_path.exists());
}

#[test]
fn test_hook_install_without_git_fails() {
    let tmp = TempDir::new().unwrap();

    // Initialize medulla WITHOUT git
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Try to install hook
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Not a git repository") || stderr.contains("git init"));
}

#[test]
fn test_hook_install_force_overwrites() {
    let tmp = TempDir::new().unwrap();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(tmp.path())
        .output()
        .unwrap();

    // Initialize medulla
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Create a custom hook
    let hooks_dir = tmp.path().join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap();
    let hook_path = hooks_dir.join("pre-commit");
    std::fs::write(&hook_path, "#!/bin/sh\necho 'Custom hook'\n").unwrap();

    // Install without --force should fail
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install"])
        .output()
        .unwrap();

    assert!(!output.status.success());

    // Install with --force should succeed
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["hook", "install", "--force"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Backed up") || stdout.contains("Installed"));

    // Verify backup was created
    let backup_path = hooks_dir.join("pre-commit.backup");
    assert!(backup_path.exists());
}

// ============================================================================
// Search Filter Integration Tests (Phase 3)
// ============================================================================

#[test]
fn test_search_with_type_filter() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add decision and task with similar titles
    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Database design"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "task", "Database setup", "--status=todo"])
        .output()
        .unwrap();

    // Search with type filter - should only find decision
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "database type:decision", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["entity_type"], "decision");
}

#[test]
fn test_search_with_status_filter() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Proposed idea", "--status=proposed"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Accepted idea", "--status=accepted"])
        .output()
        .unwrap();

    // Search with status filter
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "idea status:accepted", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "Accepted idea");
}

#[test]
fn test_search_with_tag_filter() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Backend decision", "--tag=backend"])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["add", "decision", "Frontend decision", "--tag=frontend"])
        .output()
        .unwrap();

    // Search with tag filter
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "decision tag:backend", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "Backend decision");
}

#[test]
fn test_search_combined_filters() {
    let tmp = TempDir::new().unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .output()
        .unwrap();

    // Add various decisions
    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "API Design",
            "--status=accepted",
            "--tag=api",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "API Security",
            "--status=proposed",
            "--tag=api",
        ])
        .output()
        .unwrap();

    medulla_cmd()
        .current_dir(tmp.path())
        .args([
            "add",
            "decision",
            "Database Schema",
            "--status=accepted",
            "--tag=database",
        ])
        .output()
        .unwrap();

    // Search with combined filters: search text + status + tag
    // Note: search text "API" is needed because filter-only queries don't work
    let output = medulla_cmd()
        .current_dir(tmp.path())
        .args(["search", "API status:accepted tag:api", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed.as_array().unwrap().len(), 1);
    assert_eq!(parsed[0]["title"], "API Design");
}
