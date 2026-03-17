use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fixture_dir_with_files() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("app.ts"),
        "function greet() { helper(); }\nfunction helper() {}\nclass Foo {}\nconst x = 1;\ntype Alias = string;\n",
    )
    .unwrap();
    tmp
}

fn empty_fixture_dir() -> TempDir {
    TempDir::new().unwrap()
}

#[test]
fn index_prints_summary() {
    let tmp = fixture_dir_with_files();

    Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("Indexed"))
        .stdout(predicates::str::contains("symbols"))
        .stdout(predicates::str::contains("calls"));
}

#[test]
fn index_empty_dir_prints_informational_message() {
    let tmp = empty_fixture_dir();

    Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicates::str::contains("No matching source files"));
}

#[test]
fn list_prints_all_symbols() {
    let tmp = fixture_dir_with_files();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("list")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("greet"), "should list greet: {stdout}");
    assert!(stdout.contains("helper"), "should list helper: {stdout}");
    assert!(stdout.contains("Foo"), "should list Foo: {stdout}");
    assert!(stdout.contains("Function"), "should show kind Function: {stdout}");
    assert!(stdout.contains("Class"), "should show kind Class: {stdout}");
}

#[test]
fn list_filters_by_kind() {
    let tmp = fixture_dir_with_files();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("list")
        .arg(tmp.path())
        .arg("--kind")
        .arg("Function")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("greet"), "should list greet: {stdout}");
    assert!(stdout.contains("helper"), "should list helper: {stdout}");
    assert!(!stdout.contains("Foo"), "should not list class Foo: {stdout}");
    assert!(!stdout.contains("Variable"), "should not contain Variable kind: {stdout}");
}

#[test]
fn list_empty_dir_produces_no_output() {
    let tmp = empty_fixture_dir();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("list")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    // The informational message goes to stdout from build_graph, but no symbol lines
    // after filtering. We just check it doesn't crash and the only output is the info message.
    assert!(
        !stdout.contains("Function"),
        "empty dir should produce no symbol output: {stdout}"
    );
}

fn fixture_with_calls_and_imports() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("utils.ts"),
        "export function helper() {}\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("app.ts"),
        "import { helper } from './utils';\nfunction main() { helper(); }\n",
    )
    .unwrap();
    tmp
}

#[test]
fn trace_kind_import_includes_only_import_edges() {
    let tmp = fixture_with_calls_and_imports();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "main", "--kind", "import", "--depth", "3"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("utils.ts"), "should contain imported file: {stdout}");
    assert!(stdout.contains("app.ts"), "should contain importing file: {stdout}");
    assert!(stdout.contains("Imports"), "should show import edge kind: {stdout}");
    assert!(!stdout.contains("Calls"), "should not contain call edges: {stdout}");
}

#[test]
fn trace_kind_import_excludes_call_edges() {
    let tmp = fixture_with_calls_and_imports();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "main", "--kind", "import", "--depth", "3"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(!stdout.contains("\"Calls\""), "call edges must be excluded: {stdout}");
}

#[test]
fn render_kind_import_produces_d2_with_file_nodes() {
    let tmp = fixture_with_calls_and_imports();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["render", "--kind", "import"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("utils.ts"), "should contain file node: {stdout}");
    assert!(stdout.contains("app.ts"), "should contain file node: {stdout}");
    assert!(stdout.contains("->"), "d2 uses -> for edges: {stdout}");
    assert!(stdout.contains("Imports"), "should show import edges: {stdout}");
    assert!(!stdout.contains("Calls"), "should not contain call edges: {stdout}");
}

#[test]
fn render_default_kind_is_import() {
    let tmp = fixture_with_calls_and_imports();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["render"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("->"));
}

#[test]
fn render_unknown_kind_fails() {
    let tmp = fixture_with_calls_and_imports();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["render", "--kind", "bogus"])
        .arg(tmp.path())
        .assert()
        .failure();
}
