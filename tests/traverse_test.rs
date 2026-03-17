use std::collections::HashSet;

use codegraph::graph::{CodeGraph, EdgeKind, SymbolKind, SymbolNode};
use codegraph::traverse::{trace, Direction};

fn make_symbol(name: &str) -> SymbolNode {
    SymbolNode {
        name: name.into(),
        kind: SymbolKind::Function,
        file_path: "test.ts".into(),
        line: 1,
        module: "test".into(),
    }
}

fn all_edges() -> HashSet<EdgeKind> {
    HashSet::from([EdgeKind::Calls])
}

/// Build a linear chain: a -> b -> c -> d
fn build_chain() -> (CodeGraph, Vec<petgraph::graph::NodeIndex>) {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a"));
    let b = cg.add_symbol(make_symbol("b"));
    let c = cg.add_symbol(make_symbol("c"));
    let d = cg.add_symbol(make_symbol("d"));
    cg.add_call(a, b);
    cg.add_call(b, c);
    cg.add_call(c, d);
    (cg, vec![a, b, c, d])
}

#[test]
fn default_depth_2_both_directions() {
    let (cg, nodes) = build_chain();
    let [_a, b, _c, _d] = [nodes[0], nodes[1], nodes[2], nodes[3]];

    let result = trace(&cg, b, 2, &Direction::Both, &all_edges());

    // From b at depth 2 both directions: b(0), a(1-in), c(1-out), d(2-out)
    assert_eq!(result.node_indices.len(), 4);
    assert!(result.node_indices.contains(&nodes[0])); // a
    assert!(result.node_indices.contains(&nodes[1])); // b
    assert!(result.node_indices.contains(&nodes[2])); // c
    assert!(result.node_indices.contains(&nodes[3])); // d
}

#[test]
fn depth_1_limits_reach() {
    let (cg, nodes) = build_chain();
    let b = nodes[1];

    let result = trace(&cg, b, 1, &Direction::Both, &all_edges());

    // depth 1 from b: b, a (in), c (out)
    assert_eq!(result.node_indices.len(), 3);
    assert!(result.node_indices.contains(&nodes[0]));
    assert!(result.node_indices.contains(&nodes[1]));
    assert!(result.node_indices.contains(&nodes[2]));
    assert!(!result.node_indices.contains(&nodes[3]));
}

#[test]
fn depth_3_reaches_all() {
    let (cg, nodes) = build_chain();
    let a = nodes[0];

    let result = trace(&cg, a, 3, &Direction::Out, &all_edges());

    assert_eq!(result.node_indices.len(), 4);
}

#[test]
fn direction_out_only_callees() {
    let (cg, nodes) = build_chain();
    let b = nodes[1];

    let result = trace(&cg, b, 2, &Direction::Out, &all_edges());

    // b -> c -> d
    assert!(result.node_indices.contains(&nodes[1])); // b
    assert!(result.node_indices.contains(&nodes[2])); // c
    assert!(result.node_indices.contains(&nodes[3])); // d
    assert!(!result.node_indices.contains(&nodes[0])); // no a
}

#[test]
fn direction_in_only_callers() {
    let (cg, nodes) = build_chain();
    let c = nodes[2];

    let result = trace(&cg, c, 2, &Direction::In, &all_edges());

    // c <- b <- a
    assert!(result.node_indices.contains(&nodes[2])); // c
    assert!(result.node_indices.contains(&nodes[1])); // b
    assert!(result.node_indices.contains(&nodes[0])); // a
    assert!(!result.node_indices.contains(&nodes[3])); // no d
}

#[test]
fn empty_edge_filter_returns_only_start() {
    let (cg, nodes) = build_chain();
    let b = nodes[1];

    let empty_filter: HashSet<EdgeKind> = HashSet::new();
    let result = trace(&cg, b, 2, &Direction::Both, &empty_filter);

    assert_eq!(result.node_indices.len(), 1);
    assert!(result.node_indices.contains(&b));
    assert!(result.edge_indices.is_empty());
}

#[test]
fn depth_0_returns_only_start() {
    let (cg, nodes) = build_chain();
    let b = nodes[1];

    let result = trace(&cg, b, 0, &Direction::Both, &all_edges());

    assert_eq!(result.node_indices.len(), 1);
    assert!(result.node_indices.contains(&b));
    assert!(result.edge_indices.is_empty());
}

#[test]
fn edge_indices_are_collected() {
    let (cg, nodes) = build_chain();
    let a = nodes[0];

    let result = trace(&cg, a, 2, &Direction::Out, &all_edges());

    // a->b, b->c
    assert_eq!(result.edge_indices.len(), 2);
}

#[test]
fn single_node_graph() {
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("alone"));

    let result = trace(&cg, a, 5, &Direction::Both, &all_edges());

    assert_eq!(result.node_indices.len(), 1);
    assert!(result.node_indices.contains(&a));
}

#[test]
fn diamond_graph_both_directions() {
    //   a
    //  / \
    // b   c
    //  \ /
    //   d
    let mut cg = CodeGraph::new();
    let a = cg.add_symbol(make_symbol("a"));
    let b = cg.add_symbol(make_symbol("b"));
    let c = cg.add_symbol(make_symbol("c"));
    let d = cg.add_symbol(make_symbol("d"));
    cg.add_call(a, b);
    cg.add_call(a, c);
    cg.add_call(b, d);
    cg.add_call(c, d);

    let result = trace(&cg, d, 2, &Direction::In, &all_edges());

    assert_eq!(result.node_indices.len(), 4);
}
