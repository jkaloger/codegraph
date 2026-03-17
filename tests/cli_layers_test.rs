use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fixture_with_call_chain_4() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("app.ts"),
        r#"
function a() { b(); }
function b() { c(); }
function c() { d(); }
function d() {}
"#,
    )
    .unwrap();
    tmp
}

fn fixture_with_internal_calls() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("mod.ts"),
        r#"
import { ext } from './other';
function handler() { helper(); }
function helper() {}
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("other.ts"),
        r#"
export function ext() {}
"#,
    )
    .unwrap();
    tmp
}

/// Returns the canonicalized path for a TempDir, working around macOS /var -> /private/var symlink.
fn canonical_path(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().canonicalize().unwrap()
}

fn fixture_no_internal_calls() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("mod.ts"),
        r#"
import { ext } from './other';
function handler() { ext(); }
"#,
    )
    .unwrap();
    fs::write(
        tmp.path().join("other.ts"),
        r#"
export function ext() {}
"#,
    )
    .unwrap();
    tmp
}

#[test]
fn test_trace_layers_produces_layers_output() {
    let tmp = fixture_with_call_chain_4();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "a", "--layers", "--depth", "2", "--direction", "out"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("layers: {"), "should contain layers block: {stdout}");
    assert!(stdout.contains("link: layers."), "should contain layer link: {stdout}");
}

#[test]
fn test_trace_layers_mermaid_errors() {
    let tmp = fixture_with_call_chain_4();

    Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "a", "--layers", "--format", "mermaid"])
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("layers are only supported with d2 format"));
}

#[test]
fn test_trace_without_layers_unchanged() {
    let tmp = fixture_with_call_chain_4();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["trace", "a", "--direction", "out"])
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(!stdout.contains("layers: {"), "without --layers should not contain layers block: {stdout}");
}

#[test]
fn test_render_layers_with_internal_calls() {
    let tmp = fixture_with_internal_calls();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["render", "--layers"])
        .arg(canonical_path(&tmp))
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("layers: {"), "should contain layers for file with internal calls: {stdout}");
}

#[test]
fn test_render_layers_no_internal_calls() {
    let tmp = fixture_no_internal_calls();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .args(["render", "--layers"])
        .arg(canonical_path(&tmp))
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(!stdout.contains("layers: {"), "should not contain layers (no internal calls): {stdout}");
}
