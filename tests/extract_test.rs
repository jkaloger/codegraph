use std::fs;
use tempfile::TempDir;

use codegraph::extract::{extract, ExtractionResult};
use codegraph::graph::SymbolKind;
use codegraph::parse::parse_file;

fn parse_and_extract(tmp: &TempDir, filename: &str, source: &str) -> ExtractionResult {
    let path = tmp.path().join(filename);
    fs::write(&path, source).unwrap();
    let tree = parse_file(&path).unwrap();
    extract(&tree, source, &path)
}

#[test]
fn extracts_all_five_symbol_kinds() {
    let tmp = TempDir::new().unwrap();
    let source_with_method = r#"
function greet() {}
class Foo {
    hello() {}
}
const bar = 1;
type Alias = string;
"#;

    let result = parse_and_extract(&tmp, "all_kinds.ts", source_with_method);

    let names: Vec<&str> = result.symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"greet"), "missing function greet: {:?}", names);
    assert!(names.contains(&"Foo"), "missing class Foo: {:?}", names);
    assert!(names.contains(&"hello"), "missing method hello: {:?}", names);
    assert!(names.contains(&"bar"), "missing variable bar: {:?}", names);
    assert!(names.contains(&"Alias"), "missing type alias Alias: {:?}", names);

    let greet = result.symbols.iter().find(|s| s.name == "greet").unwrap();
    assert_eq!(greet.kind, SymbolKind::Function);

    let foo = result.symbols.iter().find(|s| s.name == "Foo").unwrap();
    assert_eq!(foo.kind, SymbolKind::Class);

    let hello = result.symbols.iter().find(|s| s.name == "hello").unwrap();
    assert_eq!(hello.kind, SymbolKind::Method);

    let bar = result.symbols.iter().find(|s| s.name == "bar").unwrap();
    assert_eq!(bar.kind, SymbolKind::Variable);

    let alias = result.symbols.iter().find(|s| s.name == "Alias").unwrap();
    assert_eq!(alias.kind, SymbolKind::TypeAlias);
}

#[test]
fn extracts_correct_line_numbers() {
    let tmp = TempDir::new().unwrap();
    let source = "function a() {}\nfunction b() {}\n";

    let result = parse_and_extract(&tmp, "lines.ts", source);

    let a = result.symbols.iter().find(|s| s.name == "a").unwrap();
    assert_eq!(a.line, 1);

    let b = result.symbols.iter().find(|s| s.name == "b").unwrap();
    assert_eq!(b.line, 2);
}

#[test]
fn extracts_file_path_and_module() {
    let tmp = TempDir::new().unwrap();
    let source = "function a() {}";
    let path = tmp.path().join("mymod.ts");
    fs::write(&path, source).unwrap();
    let tree = parse_file(&path).unwrap();

    let result = extract(&tree, source, &path);

    let a = result.symbols.iter().find(|s| s.name == "a").unwrap();
    assert_eq!(a.file_path, path.to_string_lossy());
    assert_eq!(a.module, "mymod");
}

#[test]
fn extracts_call_edge() {
    let tmp = TempDir::new().unwrap();
    let source = "function a() { b(); }";

    let result = parse_and_extract(&tmp, "calls.ts", source);

    assert!(
        result.calls.iter().any(|(from, to)| from == "a" && to == "b"),
        "expected call edge (a, b), got {:?}",
        result.calls
    );
}

#[test]
fn extracts_method_call() {
    let tmp = TempDir::new().unwrap();
    let source = r#"
class Foo {
    run() {
        this.helper();
    }
    helper() {}
}
"#;

    let result = parse_and_extract(&tmp, "method_call.ts", source);

    assert!(
        result.calls.iter().any(|(from, to)| from == "run" && to == "helper"),
        "expected call edge (run, helper), got {:?}",
        result.calls
    );
}

#[test]
fn partial_parse_returns_partial_results() {
    let tmp = TempDir::new().unwrap();
    // Malformed syntax: tree-sitter will still produce a tree with errors,
    // but we should extract whatever we can.
    let source = "function good() { b(); }\nfunction ( { }}}";

    let result = parse_and_extract(&tmp, "partial.ts", source);

    assert!(
        result.symbols.iter().any(|s| s.name == "good"),
        "should extract 'good' despite parse errors: {:?}",
        result.symbols
    );
}

#[test]
fn extracts_variable_from_let_declaration() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 42;";

    let result = parse_and_extract(&tmp, "letvar.js", source);

    assert!(
        result.symbols.iter().any(|s| s.name == "x" && s.kind == SymbolKind::Variable),
        "expected variable x: {:?}",
        result.symbols
    );
}

#[test]
fn extracts_es_import() {
    let tmp = TempDir::new().unwrap();
    let source = r#"import { foo } from './utils';"#;

    let result = parse_and_extract(&tmp, "imports.ts", source);

    assert!(
        result.imports.contains(&"./utils".to_string()),
        "expected import './utils', got {:?}",
        result.imports
    );
}

#[test]
fn extracts_require_call() {
    let tmp = TempDir::new().unwrap();
    let source = r#"const fs = require('fs');"#;

    let result = parse_and_extract(&tmp, "require.js", source);

    assert!(
        result.imports.contains(&"fs".to_string()),
        "expected import 'fs', got {:?}",
        result.imports
    );
}

#[test]
fn extracts_dynamic_import() {
    let tmp = TempDir::new().unwrap();
    let source = r#"const mod = import('./lazy');"#;

    let result = parse_and_extract(&tmp, "dynamic.ts", source);

    assert!(
        result.imports.contains(&"./lazy".to_string()),
        "expected import './lazy', got {:?}",
        result.imports
    );
}

#[test]
fn extracts_reexport_specifier() {
    let tmp = TempDir::new().unwrap();
    let source = r#"export { foo } from './other';"#;

    let result = parse_and_extract(&tmp, "reexport.ts", source);

    assert!(
        result.reexports.contains(&"./other".to_string()),
        "expected reexport './other', got {:?}",
        result.reexports
    );
}

#[test]
fn local_export_has_no_reexport() {
    let tmp = TempDir::new().unwrap();
    let source = r#"export function greet() {}"#;

    let result = parse_and_extract(&tmp, "local_export.ts", source);

    assert!(
        result.reexports.is_empty(),
        "local export should not produce reexport, got {:?}",
        result.reexports
    );
}

#[test]
fn extracts_all_import_forms() {
    let tmp = TempDir::new().unwrap();
    let source = r#"
import { a } from './a';
const b = require('./b');
const c = import('./c');
export { d } from './d';
export default function greet() {}
"#;

    let result = parse_and_extract(&tmp, "all_imports.ts", source);

    assert!(result.imports.contains(&"./a".to_string()), "missing ./a: {:?}", result.imports);
    assert!(result.imports.contains(&"./b".to_string()), "missing ./b: {:?}", result.imports);
    assert!(result.imports.contains(&"./c".to_string()), "missing ./c: {:?}", result.imports);
    assert!(result.reexports.contains(&"./d".to_string()), "missing reexport ./d: {:?}", result.reexports);
    assert!(result.reexports.len() == 1, "should only have 1 reexport, got {:?}", result.reexports);
}
