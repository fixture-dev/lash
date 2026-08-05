//! Integration tests for `lash list` task-level filters
//!
//! Verifies that `--status`, `--label`, `--blocked`, and `--path` actually
//! restrict the output instead of printing the entire task tree (regression
//! test for filters being parsed but silently ignored).

#![allow(deprecated)] // for Command::cargo_bin

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn lash_cmd() -> Command {
    let mut cmd = Command::cargo_bin("lash").unwrap();
    cmd.env_remove("NO_COLOR");
    cmd
}

/// Create a project with tasks in several statuses and with labels,
/// then index it.
fn create_indexed_project() -> TempDir {
    let temp = TempDir::new().unwrap();

    fs::write(
        temp.path().join("lash.index.md"),
        "# Test Project\n\n@id: test\n\n## Tasks\n\n- [ ] Index task\n",
    )
    .unwrap();

    fs::create_dir_all(temp.path().join("tasks")).unwrap();
    fs::write(
        temp.path().join("tasks/alpha.md"),
        "# Alpha\n\n@id: alpha\n@created: 2024-01-15\n\n## Tasks\n\n\
         - [ ] Open frontend task #frontend\n\
         - [ ] Open backend task #backend\n\
         - [x] Done task\n\
         - [!] Blocked task\n",
    )
    .unwrap();

    lash_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("index")
        .assert()
        .success();

    temp
}

fn list_cmd(temp: &TempDir) -> Command {
    let mut cmd = lash_cmd();
    cmd.arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("list");
    cmd
}

#[test]
fn test_list_status_open_excludes_done_and_blocked() {
    let temp = create_indexed_project();
    list_cmd(&temp)
        .arg("--status")
        .arg("open")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Open frontend task")
                .and(predicate::str::contains("Open backend task"))
                .and(predicate::str::contains("Done task").not())
                .and(predicate::str::contains("Blocked task").not()),
        );
}

#[test]
fn test_list_status_done_excludes_open() {
    let temp = create_indexed_project();
    list_cmd(&temp)
        .arg("--status")
        .arg("done")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Done task")
                .and(predicate::str::contains("Open frontend task").not())
                .and(predicate::str::contains("Index task").not()),
        );
}

#[test]
fn test_list_blocked_shows_only_blocked_tasks() {
    let temp = create_indexed_project();
    list_cmd(&temp).arg("--blocked").assert().success().stdout(
        predicate::str::contains("Blocked task")
            .and(predicate::str::contains("Open frontend task").not())
            .and(predicate::str::contains("Done task").not()),
    );
}

#[test]
fn test_list_label_filters_tasks() {
    let temp = create_indexed_project();
    list_cmd(&temp)
        .arg("--label")
        .arg("frontend")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Open frontend task")
                .and(predicate::str::contains("Open backend task").not())
                .and(predicate::str::contains("Done task").not()),
        );
}

#[test]
fn test_list_path_prefix_filters_files() {
    let temp = create_indexed_project();
    list_cmd(&temp)
        .arg("--status")
        .arg("open")
        .arg("--path")
        .arg("tasks")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Open frontend task")
                .and(predicate::str::contains("Index task").not()),
        );
}

#[test]
fn test_list_status_json_output_contains_only_matching_tasks() {
    let temp = create_indexed_project();
    let output = list_cmd(&temp)
        .arg("--status")
        .arg("open")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert!(output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let tasks = json["tasks"].as_array().unwrap();
    assert_eq!(json["count"], tasks.len());
    assert!(!tasks.is_empty());
    for task in tasks {
        assert_eq!(task["status"], "open", "non-open task in output: {task}");
    }
}

#[test]
fn test_list_no_matching_tasks_reports_empty() {
    let temp = create_indexed_project();
    list_cmd(&temp)
        .arg("--status")
        .arg("waived")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No tasks found matching the given filters",
        ));
}

#[test]
fn test_list_without_filters_shows_all_tasks() {
    let temp = create_indexed_project();
    lash_cmd()
        .arg("--root")
        .arg(temp.path())
        .arg("--no-color")
        .arg("--tree-view")
        .arg("list")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Open frontend task")
                .and(predicate::str::contains("Done task"))
                .and(predicate::str::contains("Blocked task")),
        );
}
