use std::fs;

use assert_cmd::Command;
use tempfile::TempDir;

fn fixture_with_import() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("a.ts"),
        "import { greet } from './b';\nfunction main() { greet(); }\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("b.ts"),
        "export function greet() {}\n",
    )
    .unwrap();
    tmp
}

fn fixture_with_reexport() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("a.ts"),
        "import { greet } from './barrel';\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("barrel.ts"),
        "export { greet } from './b';\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("b.ts"),
        "export function greet() {}\n",
    )
    .unwrap();
    tmp
}

fn fixture_with_external_import() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("a.ts"),
        "import lodash from 'lodash';\nfunction main() {}\n",
    )
    .unwrap();
    tmp
}

#[test]
fn index_includes_import_edges() {
    let tmp = fixture_with_import();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("1 imports"),
        "should report 1 import edge: {stdout}"
    );
}

#[test]
fn index_includes_reexport_edges() {
    let tmp = fixture_with_reexport();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("imports"),
        "should report import edges: {stdout}"
    );
    assert!(
        stdout.contains("exports"),
        "should report export edges: {stdout}"
    );
}

#[test]
fn external_imports_produce_no_edges() {
    let tmp = fixture_with_external_import();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("0 imports"),
        "external imports should produce 0 import edges: {stdout}"
    );
    assert!(
        stdout.contains("0 exports"),
        "external imports should produce 0 export edges: {stdout}"
    );
}

fn fixture_with_references() -> TempDir {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("refs.ts"),
        "function a() { let x = 1; b(x); x = 2; }\nfunction b(val: number) {}\n",
    )
    .unwrap();
    tmp
}

#[test]
fn index_includes_reference_edges() {
    let tmp = fixture_with_references();

    let output = Command::cargo_bin("codegraph")
        .unwrap()
        .arg("index")
        .arg(tmp.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(
        stdout.contains("references"),
        "summary should include reference count: {stdout}"
    );
    assert!(
        !stdout.contains("0 references"),
        "should have >0 reference edges: {stdout}"
    );
}
