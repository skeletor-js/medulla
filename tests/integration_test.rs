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
