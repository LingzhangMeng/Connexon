//! Integration tests for the `cnx` binary.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cnx() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cnx"))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn eval_prints_the_result() {
    let output = cnx().args(["eval", "1 + 2 * 3"]).output().unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "7");
}

#[test]
fn eval_reports_syntax_errors() {
    let output = cnx().args(["eval", "1 +"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("cnx:"));
}

#[test]
fn run_functions_example() {
    let output = cnx()
        .current_dir(workspace_root())
        .args(["run", "examples/functions.cnx"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("25"));
    assert!(stdout.contains("large"));
}

#[test]
fn run_table_example() {
    let output = cnx()
        .current_dir(workspace_root())
        .args(["run", "examples/table.cnx"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("4"));
    assert!(stdout.contains("GENE0001"));
}

#[test]
fn bridge_example_runs_when_interpreters_exist() {
    let output = cnx()
        .current_dir(workspace_root())
        .args(["run", "examples/bridge.cnx"])
        .output()
        .unwrap();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("Rscript was not found") || stderr.contains("python3 was not found") {
            return;
        }
        panic!("bridge example failed: {stderr}");
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.lines().count() >= 3);
}

#[test]
fn doctor_reports_profile_and_plan() {
    let output = cnx().arg("doctor").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Connexon system profile"));
    assert!(stdout.contains("Proposed profile"));
    assert!(stdout.contains("openblas"));
}

#[test]
fn install_plan_is_written_without_installing() {
    let directory = tempfile::tempdir().unwrap();
    let output = cnx()
        .current_dir(directory.path())
        .args(["install", "--plan-only"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let plan = std::fs::read_to_string(directory.path().join("connexon.install-plan.json"))
        .expect("install plan file");
    assert!(plan.contains("\"profile_kind\""));
    assert!(plan.contains("\"components\""));
    assert!(plan.contains("openblas"));
}
