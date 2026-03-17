use std::fs;
use tempfile::TempDir;

use codegraph::parse::parse_file;

#[test]
fn parses_valid_typescript_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("index.ts");
    fs::write(&path, "function greet(name: string): string { return name; }").unwrap();

    let tree = parse_file(&path).unwrap();

    assert!(!tree.root_node().has_error());
    assert_eq!(tree.root_node().kind(), "program");
}

#[test]
fn parses_valid_tsx_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("app.tsx");
    fs::write(&path, "const App = () => <div>hello</div>;").unwrap();

    let tree = parse_file(&path).unwrap();

    assert!(!tree.root_node().has_error());
}

#[test]
fn parses_valid_javascript_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("util.js");
    fs::write(&path, "function add(a, b) { return a + b; }").unwrap();

    let tree = parse_file(&path).unwrap();

    assert!(!tree.root_node().has_error());
}

#[test]
fn parses_valid_jsx_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("button.jsx");
    fs::write(&path, "const Btn = () => <button>click</button>;").unwrap();

    let tree = parse_file(&path).unwrap();

    assert!(!tree.root_node().has_error());
}

#[test]
fn returns_tree_with_errors_for_broken_syntax() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("broken.ts");
    fs::write(&path, "function greet(name: string { return }}}}}").unwrap();

    let tree = parse_file(&path).unwrap();

    assert!(tree.root_node().has_error());
}

#[test]
fn errors_on_unsupported_extension() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("style.css");
    fs::write(&path, "body { color: red; }").unwrap();

    let result = parse_file(&path);

    assert!(result.is_err());
}

#[test]
fn errors_on_missing_file() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nonexistent.ts");

    let result = parse_file(&path);

    assert!(result.is_err());
}
