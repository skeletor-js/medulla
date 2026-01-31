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
