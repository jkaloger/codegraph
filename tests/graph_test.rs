use codegraph::graph::{CodeGraph, EdgeKind, SymbolKind, SymbolNode};

fn make_symbol(name: &str) -> SymbolNode {
    SymbolNode {
        name: name.into(),
        kind: SymbolKind::Variable,
        file_path: "src/index.ts".into(),
        line: 1,
        module: "index".into(),
    }
}

#[test]
fn add_reference_inserts_reads_from_edge() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("x"));
    let b = cg.add_symbol(make_symbol("doStuff"));

    cg.add_reference(a, b, EdgeKind::ReadsFrom);

    assert_eq!(cg.graph.edge_count(), 1);
    let edge = &cg.graph[cg.graph.edge_indices().next().unwrap()];
    assert_eq!(*edge, EdgeKind::ReadsFrom);
}

#[test]
fn add_reference_inserts_writes_to_edge() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("init"));
    let b = cg.add_symbol(make_symbol("x"));

    cg.add_reference(a, b, EdgeKind::WritesTo);

    assert_eq!(cg.graph.edge_count(), 1);
    let edge = &cg.graph[cg.graph.edge_indices().next().unwrap()];
    assert_eq!(*edge, EdgeKind::WritesTo);
}

#[test]
#[should_panic(expected = "not a reference edge kind")]
fn add_reference_rejects_call_edge() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("foo"));
    let b = cg.add_symbol(make_symbol("bar"));

    cg.add_reference(a, b, EdgeKind::Calls);
}

#[test]
fn references_of_returns_only_reference_edges() {
    let mut cg = CodeGraph::new();
    let x = cg.add_symbol(make_symbol("x"));
    let reader = cg.add_symbol(make_symbol("doStuff"));
    let writer = cg.add_symbol(make_symbol("init"));
    let caller = cg.add_symbol(make_symbol("main"));

    cg.add_reference(reader, x, EdgeKind::ReadsFrom);
    cg.add_reference(writer, x, EdgeKind::WritesTo);
    cg.add_call(caller, x);

    assert_eq!(cg.graph.edge_count(), 3);

    let refs = cg.references_of(x);
    assert_eq!(refs.len(), 2);

    let kinds: Vec<_> = refs.iter().map(|(_, k)| *k).collect();
    assert!(kinds.contains(&&EdgeKind::ReadsFrom));
    assert!(kinds.contains(&&EdgeKind::WritesTo));
}

#[test]
fn references_of_returns_empty_when_no_references() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("foo"));
    let b = cg.add_symbol(make_symbol("bar"));

    cg.add_call(a, b);

    let refs = cg.references_of(b);
    assert!(refs.is_empty());
}
