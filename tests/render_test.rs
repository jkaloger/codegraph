use std::collections::HashSet;

use codegraph::graph::{CodeGraph, EdgeKind, SymbolKind, SymbolNode};
use codegraph::render::{render, render_d2, render_mermaid, Format};
use codegraph::traverse::{trace, Direction};

fn make_symbol(name: &str, kind: SymbolKind, file_path: &str, line: usize) -> SymbolNode {
    SymbolNode {
        name: name.into(),
        kind,
        file_path: file_path.into(),
        line,
        module: "test".into(),
    }
}

fn all_edges() -> HashSet<EdgeKind> {
    HashSet::from([EdgeKind::Calls])
}

#[test]
fn render_single_node() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let d2 = render_d2(&result, &cg);

    assert!(d2.contains("node_0"));
    assert!(d2.contains("greet"));
    assert!(d2.contains("Function"));
    assert!(d2.contains("src/app.ts:5"));
}

#[test]
fn render_edge_label_contains_calls() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));
    let b = cg.add_symbol(make_symbol("helper", SymbolKind::Function, "src/utils.ts", 10));
    cg.add_call(a, b);

    let result = trace(&cg, a, 1, &Direction::Out, &all_edges());
    let d2 = render_d2(&result, &cg);

    assert!(d2.contains("node_0 -> node_1"));
    assert!(d2.contains("Calls"));
}

#[test]
fn render_node_labels_multiline() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("MyClass", SymbolKind::Class, "src/models.ts", 20));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let d2 = render_d2(&result, &cg);

    // Label should contain name, kind, file:line separated by \n
    assert!(d2.contains(r"MyClass\nClass\nsrc/models.ts:20"));
}

#[test]
fn render_includes_all_traced_nodes_and_edges() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));
    let b = cg.add_symbol(make_symbol("b", SymbolKind::Method, "b.ts", 5));
    let c = cg.add_symbol(make_symbol("c", SymbolKind::Variable, "c.ts", 10));
    cg.add_call(a, b);
    cg.add_call(b, c);

    let result = trace(&cg, a, 2, &Direction::Out, &all_edges());
    let d2 = render_d2(&result, &cg);

    assert!(d2.contains("node_0"));
    assert!(d2.contains("node_1"));
    assert!(d2.contains("node_2"));
    assert!(d2.contains("node_0 -> node_1"));
    assert!(d2.contains("node_1 -> node_2"));
}

#[test]
fn write_output_to_file() {
    use codegraph::render::write_output;

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("output.d2");

    write_output("hello d2", Some(file_path.as_path())).unwrap();

    let contents = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(contents, "hello d2");
}

#[test]
fn render_empty_trace_produces_no_output() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let d2 = render_d2(&result, &cg);

    // Should have the node but no edges
    assert!(d2.contains("node_0"));
    assert!(!d2.contains("->"));
}

#[test]
fn mermaid_starts_with_flowchart_td() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let mmd = render_mermaid(&result, &cg);

    assert!(mmd.starts_with("flowchart TD"));
}

#[test]
fn mermaid_single_node_label() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let mmd = render_mermaid(&result, &cg);

    assert!(mmd.contains("node_0[\"greet<br/>Function<br/>src/app.ts:5\"]"));
}

#[test]
fn mermaid_edge_format() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));
    let b = cg.add_symbol(make_symbol("helper", SymbolKind::Function, "src/utils.ts", 10));
    cg.add_call(a, b);

    let result = trace(&cg, a, 1, &Direction::Out, &all_edges());
    let mmd = render_mermaid(&result, &cg);

    assert!(mmd.contains("node_0 -->|\"Calls\"| node_1"));
}

#[test]
fn mermaid_includes_all_traced_nodes_and_edges() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));
    let b = cg.add_symbol(make_symbol("b", SymbolKind::Method, "b.ts", 5));
    let c = cg.add_symbol(make_symbol("c", SymbolKind::Variable, "c.ts", 10));
    cg.add_call(a, b);
    cg.add_call(b, c);

    let result = trace(&cg, a, 2, &Direction::Out, &all_edges());
    let mmd = render_mermaid(&result, &cg);

    assert!(mmd.contains("node_0"));
    assert!(mmd.contains("node_1"));
    assert!(mmd.contains("node_2"));
    assert!(mmd.contains("node_0 -->|\"Calls\"| node_1"));
    assert!(mmd.contains("node_1 -->|\"Calls\"| node_2"));
}

#[test]
fn mermaid_empty_trace_no_edges() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let mmd = render_mermaid(&result, &cg);

    assert!(mmd.contains("node_0"));
    assert!(!mmd.contains("-->"));
}

#[test]
fn render_dispatcher_selects_d2() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let via_dispatcher = render(&result, &cg, Format::D2);
    let via_direct = render_d2(&result, &cg);

    assert_eq!(via_dispatcher, via_direct);
}

#[test]
fn render_dispatcher_selects_mermaid() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("greet", SymbolKind::Function, "src/app.ts", 5));

    let result = trace(&cg, a, 0, &Direction::Out, &all_edges());
    let via_dispatcher = render(&result, &cg, Format::Mermaid);
    let via_direct = render_mermaid(&result, &cg);

    assert_eq!(via_dispatcher, via_direct);
}
