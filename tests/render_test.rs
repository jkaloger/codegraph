use std::collections::HashSet;

use codegraph::graph::{CodeGraph, EdgeKind, SymbolKind, SymbolNode};
use codegraph::render::{render, render_d2, render_d2_file_layers, render_d2_layers, render_mermaid, Format};
use codegraph::traverse::{trace, Direction, TraceResult};

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
    assert!(d2.contains("[call]"));
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

    assert!(mmd.contains("node_0 -->|\"[call]\"| node_1"));
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
    assert!(mmd.contains("node_0 -->|\"[call]\"| node_1"));
    assert!(mmd.contains("node_1 -->|\"[call]\"| node_2"));
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

#[test]
fn test_render_d2_layers_produces_layers_block() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));
    let b = cg.add_symbol(make_symbol("b", SymbolKind::Function, "b.ts", 1));
    let c = cg.add_symbol(make_symbol("c", SymbolKind::Function, "c.ts", 1));
    let d = cg.add_symbol(make_symbol("d", SymbolKind::Function, "d.ts", 1));
    cg.add_call(a, b);
    cg.add_call(b, c);
    cg.add_call(c, d);

    let result = trace(&cg, a, 2, &Direction::Out, &all_edges());
    let output = render_d2_layers(&result, &cg, a, 2, &Direction::Out, &all_edges());

    assert!(output.contains("layers: {"), "should contain layers block: {output}");
    assert!(output.contains("link: layers."), "should contain layer link: {output}");
}

#[test]
fn test_render_d2_layers_suppresses_redundant() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a", SymbolKind::Function, "a.ts", 1));
    let b = cg.add_symbol(make_symbol("b", SymbolKind::Function, "b.ts", 1));
    cg.add_call(a, b);

    let result = trace(&cg, a, 1, &Direction::Out, &all_edges());
    let output = render_d2_layers(&result, &cg, a, 1, &Direction::Out, &all_edges());

    assert!(!output.contains("layers: {"), "should not contain layers block (redundant): {output}");
}

#[test]
fn test_render_d2_file_layers_produces_drill_down() {
    let mut cg = CodeGraph::new();
    let sym_a = cg.add_symbol(make_symbol("funcA", SymbolKind::Function, "src/mod.ts", 1));
    let sym_b = cg.add_symbol(make_symbol("funcB", SymbolKind::Function, "src/mod.ts", 10));
    cg.add_call(sym_a, sym_b);

    let file_node = cg.add_file("src/mod.ts");
    let other_file = cg.add_file("src/other.ts");
    cg.add_import(file_node, other_file);

    let mut node_indices = HashSet::new();
    node_indices.insert(file_node);
    node_indices.insert(other_file);

    let mut edge_indices = HashSet::new();
    for edge in cg.graph.edge_references() {
        use petgraph::visit::EdgeRef;
        if *edge.weight() == EdgeKind::Imports {
            if node_indices.contains(&edge.source()) && node_indices.contains(&edge.target()) {
                edge_indices.insert(edge.id());
            }
        }
    }

    let trace_result = TraceResult {
        node_indices,
        edge_indices,
    };

    let output = render_d2_file_layers(&trace_result, &cg);

    assert!(output.contains("layers: {"), "should contain layers block: {output}");
    assert!(output.contains("link: layers."), "should contain layer link for file: {output}");
    assert!(output.contains("funcA"), "layer should contain funcA: {output}");
    assert!(output.contains("funcB"), "layer should contain funcB: {output}");
}

#[test]
fn test_render_d2_file_layers_no_layer_for_empty_file() {
    let mut cg = CodeGraph::new();
    let sym_a = cg.add_symbol(make_symbol("funcA", SymbolKind::Function, "src/mod.ts", 1));
    let sym_b = cg.add_symbol(make_symbol("funcB", SymbolKind::Function, "src/other.ts", 1));

    let file_a = cg.add_file("src/mod.ts");
    let file_b = cg.add_file("src/other.ts");
    cg.add_import(file_a, file_b);

    let mut node_indices = HashSet::new();
    node_indices.insert(file_a);
    node_indices.insert(file_b);

    let mut edge_indices = HashSet::new();
    for edge in cg.graph.edge_references() {
        use petgraph::visit::EdgeRef;
        if *edge.weight() == EdgeKind::Imports {
            edge_indices.insert(edge.id());
        }
    }

    let trace_result = TraceResult {
        node_indices,
        edge_indices,
    };

    let output = render_d2_file_layers(&trace_result, &cg);

    assert!(!output.contains("layers: {"), "should not contain layers (no internal calls): {output}");

    // suppress unused warnings
    let _ = sym_a;
    let _ = sym_b;
}
