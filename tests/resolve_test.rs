use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

use codegraph::resolve::{resolve_specifier, ResolveResult};

fn canonical(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

fn expected(p: &Path) -> ResolveResult {
    ResolveResult::Resolved(canonical(p))
}

#[test]
fn resolves_relative_with_ts_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo.ts"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./foo", &importer);

    assert_eq!(result, expected(&dir.path().join("foo.ts")));
}

#[test]
fn resolves_relative_with_tsx_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("component.tsx"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./component", &importer);

    assert_eq!(result, expected(&dir.path().join("component.tsx")));
}

#[test]
fn resolves_relative_with_js_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("util.js"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./util", &importer);

    assert_eq!(result, expected(&dir.path().join("util.js")));
}

#[test]
fn resolves_relative_with_jsx_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("widget.jsx"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./widget", &importer);

    assert_eq!(result, expected(&dir.path().join("widget.jsx")));
}

#[test]
fn resolves_index_file_in_directory() {
    let dir = tempdir().unwrap();
    let bar_dir = dir.path().join("bar");
    fs::create_dir(&bar_dir).unwrap();
    fs::write(bar_dir.join("index.tsx"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./bar", &importer);

    assert_eq!(result, expected(&bar_dir.join("index.tsx")));
}

#[test]
fn resolves_index_ts_before_index_tsx() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("index.ts"), "").unwrap();
    fs::write(sub.join("index.tsx"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./sub", &importer);

    assert_eq!(result, expected(&sub.join("index.ts")));
}

#[test]
fn resolves_exact_path_first() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo"), "").unwrap();
    fs::write(dir.path().join("foo.ts"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./foo", &importer);

    assert_eq!(result, expected(&dir.path().join("foo")));
}

#[test]
fn resolves_parent_relative_path() {
    let dir = tempdir().unwrap();
    let sub = dir.path().join("sub");
    fs::create_dir(&sub).unwrap();
    fs::write(dir.path().join("lib.ts"), "").unwrap();

    let importer = sub.join("main.ts");
    let result = resolve_specifier("../lib", &importer);

    assert_eq!(result, expected(&dir.path().join("lib.ts")));
}

#[test]
fn bare_specifier_returns_external() {
    let dir = tempdir().unwrap();
    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("lodash", &importer);

    assert_eq!(result, ResolveResult::External("lodash".to_string()));
}

#[test]
fn scoped_package_returns_external() {
    let dir = tempdir().unwrap();
    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("@scope/pkg", &importer);

    assert_eq!(result, ResolveResult::External("@scope/pkg".to_string()));
}

#[test]
fn resolves_specifier_with_explicit_extension() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("foo.ts"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./foo.ts", &importer);

    assert_eq!(result, expected(&dir.path().join("foo.ts")));
}

#[test]
fn ts_extension_preferred_over_tsx() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("comp.ts"), "").unwrap();
    fs::write(dir.path().join("comp.tsx"), "").unwrap();

    let importer = dir.path().join("main.ts");
    let result = resolve_specifier("./comp", &importer);

    assert_eq!(result, expected(&dir.path().join("comp.ts")));
}
