use std::fs;
use tempfile::TempDir;

use std::collections::HashMap;

use codegraph::extract::{extract, ExtractionResult, RefKind, ReferenceEntry};
use codegraph::graph::{CodeGraph, EdgeKind, SymbolKind};
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

fn refs_for<'a>(result: &'a ExtractionResult, name: &str) -> Vec<&'a ReferenceEntry> {
    result.references.iter().filter(|r| r.symbol_name == name).collect()
}

fn scoped_refs_for<'a>(result: &'a ExtractionResult, name: &str, scope: &str) -> Vec<&'a ReferenceEntry> {
    result.references.iter()
        .filter(|r| r.symbol_name == name && r.enclosing_scope.as_deref() == Some(scope))
        .collect()
}

fn ref_kinds_for<'a>(result: &'a ExtractionResult, name: &str) -> Vec<&'a RefKind> {
    refs_for(result, name).iter().map(|r| &r.kind).collect()
}

#[test]
fn reference_decl_read_reassign() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 1; console.log(x); x = 2;";
    let result = parse_and_extract(&tmp, "refs.ts", source);

    let x_refs = refs_for(&result, "x");
    let writes: Vec<_> = x_refs.iter().filter(|r| r.kind == RefKind::Write).collect();
    let reads: Vec<_> = x_refs.iter().filter(|r| r.kind == RefKind::Read).collect();

    assert_eq!(writes.len(), 2, "expected 2 writes for x (decl + reassign), got {:?}", x_refs);
    assert_eq!(reads.len(), 1, "expected 1 read for x (log arg), got {:?}", x_refs);
}

#[test]
fn destructuring_produces_write_refs_and_rhs_read() {
    let tmp = TempDir::new().unwrap();
    let source = "const { a, b } = obj;";
    let result = parse_and_extract(&tmp, "destructure.ts", source);

    let a_kinds = ref_kinds_for(&result, "a");
    let b_kinds = ref_kinds_for(&result, "b");
    let obj_kinds = ref_kinds_for(&result, "obj");

    assert!(a_kinds.contains(&&RefKind::Write), "expected write ref for a, got {:?}", a_kinds);
    assert!(b_kinds.contains(&&RefKind::Write), "expected write ref for b, got {:?}", b_kinds);
    assert!(obj_kinds.contains(&&RefKind::Read), "expected read ref for obj, got {:?}", obj_kinds);
}

#[test]
fn call_target_not_double_counted_as_read() {
    let tmp = TempDir::new().unwrap();
    let source = "function caller() { foo(); }";
    let result = parse_and_extract(&tmp, "call_target.ts", source);

    let foo_refs = refs_for(&result, "foo");
    assert!(
        foo_refs.is_empty(),
        "call target foo should not appear as a read reference, got {:?}",
        foo_refs
    );
}

#[test]
fn augmented_assignment_is_write() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 0; x += 1;";
    let result = parse_and_extract(&tmp, "augmented.ts", source);

    let writes: Vec<_> = refs_for(&result, "x")
        .into_iter()
        .filter(|r| r.kind == RefKind::Write)
        .collect();
    assert_eq!(writes.len(), 2, "expected 2 writes (decl + +=), got {:?}", refs_for(&result, "x"));
}

#[test]
fn update_expression_is_write() {
    let tmp = TempDir::new().unwrap();
    let source = "let i = 0; i++;";
    let result = parse_and_extract(&tmp, "update.ts", source);

    let writes: Vec<_> = refs_for(&result, "i")
        .into_iter()
        .filter(|r| r.kind == RefKind::Write)
        .collect();
    assert_eq!(writes.len(), 2, "expected 2 writes (decl + ++), got {:?}", refs_for(&result, "i"));
}

#[test]
fn function_arg_is_read() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 1; let y = x + 2;";
    let result = parse_and_extract(&tmp, "read_expr.ts", source);

    let x_reads: Vec<_> = refs_for(&result, "x")
        .into_iter()
        .filter(|r| r.kind == RefKind::Read)
        .collect();
    assert_eq!(x_reads.len(), 1, "expected 1 read for x in RHS, got {:?}", refs_for(&result, "x"));
}

#[test]
fn array_destructuring_produces_write_refs() {
    let tmp = TempDir::new().unwrap();
    let source = "const [a, b] = arr;";
    let result = parse_and_extract(&tmp, "array_destructure.ts", source);

    assert!(ref_kinds_for(&result, "a").contains(&&RefKind::Write), "expected write for a");
    assert!(ref_kinds_for(&result, "b").contains(&&RefKind::Write), "expected write for b");
    assert!(ref_kinds_for(&result, "arr").contains(&&RefKind::Read), "expected read for arr");
}

#[test]
fn references_track_enclosing_scope() {
    let tmp = TempDir::new().unwrap();
    let source = "function a() { let x = 1; b(x); x = 2; }";
    let result = parse_and_extract(&tmp, "scope.ts", source);

    let x_reads_in_a = scoped_refs_for(&result, "x", "a");
    assert_eq!(
        x_reads_in_a.iter().filter(|r| r.kind == RefKind::Read).count(),
        1,
        "expected 1 read of x inside a, got {:?}", x_reads_in_a
    );

    let x_writes_in_a = scoped_refs_for(&result, "x", "a");
    assert_eq!(
        x_writes_in_a.iter().filter(|r| r.kind == RefKind::Write).count(),
        2,
        "expected 2 writes of x inside a (decl + reassign), got {:?}", x_writes_in_a
    );
}

fn build_graph_from_source(tmp: &TempDir, filename: &str, source: &str) -> (CodeGraph, HashMap<String, petgraph::graph::NodeIndex>) {
    let result = parse_and_extract(tmp, filename, source);
    let mut graph = CodeGraph::new();
    let mut symbol_indices = HashMap::new();

    for symbol in result.symbols {
        let name = symbol.name.clone();
        let idx = graph.add_symbol(symbol);
        symbol_indices.insert(name, idx);
    }

    for (caller, callee) in result.calls {
        if let (Some(&from), Some(&to)) = (symbol_indices.get(&caller), symbol_indices.get(&callee)) {
            graph.add_call(from, to);
        }
    }

    for reference in &result.references {
        let Some(scope) = &reference.enclosing_scope else { continue };
        let Some(&from) = symbol_indices.get(scope) else { continue };
        let Some(&to) = symbol_indices.get(&reference.symbol_name) else { continue };
        let edge_kind = match reference.kind {
            RefKind::Read => EdgeKind::ReadsFrom,
            RefKind::Write => EdgeKind::WritesTo,
        };
        graph.add_reference(from, to, edge_kind);
    }

    (graph, symbol_indices)
}

#[test]
fn graph_contains_reference_edges_for_read_and_write() {
    let tmp = TempDir::new().unwrap();
    let source = "function a() { let x = 1; b(x); x = 2; }\nfunction b(val: number) {}";
    let (graph, indices) = build_graph_from_source(&tmp, "graph_refs.ts", source);

    let a_idx = indices["a"];
    let x_idx = indices["x"];

    let ref_edges: Vec<_> = graph.graph.edges_connecting(a_idx, x_idx)
        .filter(|e| matches!(e.weight(), EdgeKind::ReadsFrom | EdgeKind::WritesTo))
        .collect();

    let reads: Vec<_> = ref_edges.iter().filter(|e| *e.weight() == EdgeKind::ReadsFrom).collect();
    let writes: Vec<_> = ref_edges.iter().filter(|e| *e.weight() == EdgeKind::WritesTo).collect();

    assert_eq!(reads.len(), 1, "expected 1 ReadsFrom edge a->x, got {}", reads.len());
    assert_eq!(writes.len(), 2, "expected 2 WritesTo edges a->x (decl + reassign), got {}", writes.len());
}

#[test]
fn module_scope_references_produce_no_graph_edges() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 1; x = 2;";
    let (graph, _) = build_graph_from_source(&tmp, "no_scope_refs.ts", source);

    let ref_count = graph.graph.edge_references()
        .filter(|e| matches!(e.weight(), EdgeKind::ReadsFrom | EdgeKind::WritesTo))
        .count();
    assert_eq!(ref_count, 0, "module-scope references should not produce edges");
}

#[test]
fn module_scope_references_have_no_enclosing_scope() {
    let tmp = TempDir::new().unwrap();
    let source = "let x = 1; x = 2;";
    let result = parse_and_extract(&tmp, "module_scope.ts", source);

    let x_refs = refs_for(&result, "x");
    assert!(
        x_refs.iter().all(|r| r.enclosing_scope.is_none()),
        "module-scope references should have no enclosing scope, got {:?}", x_refs
    );
}
