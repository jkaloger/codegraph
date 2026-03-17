use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fixture_with_call_chain() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("app.ts"),
        r#"
function main() { process(); }
function process() { validate(); }
function validate() {}
"#,
    )
    .unwrap();
    tmp
}

#[test]
fn trace_default_d2_depth_2_both() {
    let tmp = fixture_with_call_chain();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "process"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("process"), "should contain traced symbol: {stdout}");
    assert!(stdout.contains("main"), "should contain caller (depth 1 in): {stdout}");
    assert!(stdout.contains("validate"), "should contain callee (depth 1 out): {stdout}");
    assert!(stdout.contains("->"), "d2 uses -> for edges: {stdout}");
}

#[test]
fn trace_depth_3_reaches_full_chain() {
    let tmp = fixture_with_call_chain();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "main", "--depth", "3"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("main"), "{stdout}");
    assert!(stdout.contains("process"), "{stdout}");
    assert!(stdout.contains("validate"), "{stdout}");
}

#[test]
fn trace_direction_in_only_callers() {
    let tmp = fixture_with_call_chain();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "validate", "--direction", "in", "--depth", "3"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("validate"), "{stdout}");
    assert!(stdout.contains("process"), "should contain caller: {stdout}");
    assert!(stdout.contains("main"), "should contain transitive caller: {stdout}");
}

#[test]
fn trace_direction_out_only_callees() {
    let tmp = fixture_with_call_chain();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "main", "--direction", "out", "--depth", "3"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("main"), "{stdout}");
    assert!(stdout.contains("process"), "{stdout}");
    assert!(stdout.contains("validate"), "{stdout}");
}

#[test]
fn trace_kind_call_excludes_nothing_for_now() {
    let tmp = fixture_with_call_chain();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "process", "--kind", "call"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("process"));
}

#[test]
fn trace_kind_all_traverses_all() {
    let tmp = fixture_with_call_chain();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "process", "--kind", "all"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("process"));
}

#[test]
fn trace_format_mermaid() {
    let tmp = fixture_with_call_chain();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "process", "--format", "mermaid"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("flowchart TD"));
}

#[test]
fn trace_output_writes_to_file() {
    let tmp = fixture_with_call_chain();
    let out_file = tmp.path().join("graph.d2");

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "process", "--output"])
        .arg(&out_file)
        .arg(tmp.path())
        .assert()
        .success();

    let content = fs::read_to_string(&out_file).expect("output file should exist");
    assert!(content.contains("process"), "file should contain trace output: {content}");
}

#[test]
fn trace_symbol_not_found_exit_1() {
    let tmp = fixture_with_call_chain();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "nonexistent"])
        .arg(tmp.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn trace_ambiguous_symbol_exit_1() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("a.ts"),
        "function dupe() {}\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("b.ts"),
        "function dupe() {}\n",
    )
    .unwrap();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "dupe"])
        .arg(tmp.path())
        .assert()
        .failure()
        .code(1);

    let stderr = String::from_utf8(output.get_output().stderr.clone()).unwrap();
    assert!(stderr.contains("dupe"), "should list ambiguous matches: {stderr}");
}
