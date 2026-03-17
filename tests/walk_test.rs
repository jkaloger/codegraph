use std::fs;
use tempfile::TempDir;

use codegraph::walk::discover_files;

#[test]
fn discovers_ts_js_files_in_nested_dirs() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("src/components")).unwrap();
    fs::write(root.join("index.ts"), "").unwrap();
    fs::write(root.join("app.tsx"), "").unwrap();
    fs::write(root.join("src/util.js"), "").unwrap();
    fs::write(root.join("src/components/button.jsx"), "").unwrap();

    // non-matching files should be ignored
    fs::write(root.join("README.md"), "").unwrap();
    fs::write(root.join("src/style.css"), "").unwrap();

    let mut files = discover_files(root).unwrap();
    files.sort();

    let mut expected = vec![
        root.join("index.ts"),
        root.join("app.tsx"),
        root.join("src/util.js"),
        root.join("src/components/button.jsx"),
    ];
    expected.sort();

    assert_eq!(files, expected);
}

#[test]
fn skips_node_modules() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    fs::create_dir_all(root.join("node_modules/lodash")).unwrap();
    fs::write(root.join("node_modules/lodash/index.js"), "").unwrap();
    fs::write(root.join("index.ts"), "").unwrap();

    let files = discover_files(root).unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0], root.join("index.ts"));
}

#[test]
fn empty_dir_returns_empty_vec() {
    let tmp = TempDir::new().unwrap();

    let files = discover_files(tmp.path()).unwrap();

    assert!(files.is_empty());
}

#[test]
fn dir_with_no_matching_extensions_returns_empty_vec() {
    let tmp = TempDir::new().unwrap();

    fs::write(tmp.path().join("readme.md"), "").unwrap();
    fs::write(tmp.path().join("config.json"), "").unwrap();

    let files = discover_files(tmp.path()).unwrap();

    assert!(files.is_empty());
}
