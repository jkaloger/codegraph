use std::collections::HashSet;
use std::fmt::Write;
use std::io;
use std::path::Path;
use std::str::FromStr;

use petgraph::graph::NodeIndex;

use crate::graph::{CodeGraph, EdgeKind, NodeKind};
use crate::traverse::{self, Direction, TraceResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    D2,
    Mermaid,
}

impl FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "d2" => Ok(Format::D2),
            "mermaid" => Ok(Format::Mermaid),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}

pub fn render(trace: &TraceResult, graph: &CodeGraph, format: Format) -> String {
    match format {
        Format::D2 => render_d2(trace, graph),
        Format::Mermaid => render_mermaid(trace, graph),
    }
}

pub fn render_d2(trace: &TraceResult, graph: &CodeGraph) -> String {
    let mut output = String::new();

    let mut node_indices: Vec<_> = trace.node_indices.iter().copied().collect();
    node_indices.sort_by_key(|n| n.index());

    for idx in &node_indices {
        match &graph.graph[*idx] {
            NodeKind::Symbol(node) => {
                writeln!(
                    output,
                    "node_{}: \"{}\\n{}\\n{}:{}\"",
                    idx.index(),
                    node.name,
                    node.kind,
                    node.file_path,
                    node.line,
                )
                .unwrap();
            }
            NodeKind::File(file) => {
                writeln!(
                    output,
                    "node_{}: \"{}\"",
                    idx.index(),
                    file.file_path,
                )
                .unwrap();
            }
        }
    }

    let mut edge_indices: Vec<_> = trace.edge_indices.iter().copied().collect();
    edge_indices.sort_by_key(|e| e.index());

    for edge_idx in &edge_indices {
        let (source, target) = graph.graph.edge_endpoints(*edge_idx).unwrap();
        let weight = &graph.graph[*edge_idx];
        writeln!(
            output,
            "node_{} -> node_{}: \"{}\"",
            source.index(),
            target.index(),
            weight,
        )
        .unwrap();
    }

    output.trim_end().to_string()
}

pub fn render_d2_layers(
    trace: &TraceResult,
    graph: &CodeGraph,
    start: NodeIndex,
    _depth: usize,
    direction: &Direction,
    edge_filter: &HashSet<EdgeKind>,
) -> String {
    let mut output = String::new();

    let mut node_indices: Vec<_> = trace.node_indices.iter().copied().collect();
    node_indices.sort_by_key(|n| n.index());

    let mut edge_indices: Vec<_> = trace.edge_indices.iter().copied().collect();
    edge_indices.sort_by_key(|e| e.index());

    // Determine which nodes are leaf nodes (no further expansion possible).
    // A leaf is the start node itself (we don't recurse into it) or a node
    // at the fringe of the BFS (depth == requested depth).
    // We identify non-leaf, non-start symbol nodes and run depth-1 traces.
    let mut layers: Vec<(NodeIndex, String, TraceResult)> = Vec::new();

    for &idx in &node_indices {
        if idx == start {
            continue;
        }
        if !matches!(&graph.graph[idx], NodeKind::Symbol(_)) {
            continue;
        }

        let sub_trace = traverse::trace(graph, idx, 1, direction, edge_filter);

        // AC-3: skip if sub_trace edges are all already in the root trace
        let has_new_edges = sub_trace
            .edge_indices
            .iter()
            .any(|e| !trace.edge_indices.contains(e));

        if !has_new_edges {
            continue;
        }

        let layer_id = sanitise_layer_id(idx, graph);
        layers.push((idx, layer_id, sub_trace));
    }

    let layer_node_ids: HashSet<NodeIndex> = layers.iter().map(|(idx, _, _)| *idx).collect();

    // Root board nodes
    for &idx in &node_indices {
        let has_layer = layer_node_ids.contains(&idx);
        render_d2_node(&mut output, idx, graph, "", has_layer, &layers);
    }

    // Root board edges
    for &edge_idx in &edge_indices {
        let (source, target) = graph.graph.edge_endpoints(edge_idx).unwrap();
        let weight = &graph.graph[edge_idx];
        writeln!(
            output,
            "node_{} -> node_{}: \"{}\"",
            source.index(),
            target.index(),
            weight,
        )
        .unwrap();
    }

    // Layers block
    if !layers.is_empty() {
        writeln!(output).unwrap();
        writeln!(output, "layers: {{").unwrap();

        for (_, layer_id, sub_trace) in &layers {
            writeln!(output, "  {}: {{", layer_id).unwrap();

            let mut sub_nodes: Vec<_> = sub_trace.node_indices.iter().copied().collect();
            sub_nodes.sort_by_key(|n| n.index());

            for &idx in &sub_nodes {
                render_d2_node(&mut output, idx, graph, "    ", false, &[]);
            }

            let mut sub_edges: Vec<_> = sub_trace.edge_indices.iter().copied().collect();
            sub_edges.sort_by_key(|e| e.index());

            for &edge_idx in &sub_edges {
                let (source, target) = graph.graph.edge_endpoints(edge_idx).unwrap();
                let weight = &graph.graph[edge_idx];
                writeln!(
                    output,
                    "    node_{} -> node_{}: \"{}\"",
                    source.index(),
                    target.index(),
                    weight,
                )
                .unwrap();
            }

            writeln!(output, "  }}").unwrap();
        }

        writeln!(output, "}}").unwrap();
    }

    output.trim_end().to_string()
}

pub fn render_d2_file_layers(
    trace: &TraceResult,
    graph: &CodeGraph,
) -> String {
    use petgraph::visit::EdgeRef;

    let mut output = String::new();

    let mut node_indices: Vec<_> = trace.node_indices.iter().copied().collect();
    node_indices.sort_by_key(|n| n.index());

    let mut edge_indices: Vec<_> = trace.edge_indices.iter().copied().collect();
    edge_indices.sort_by_key(|e| e.index());

    // For each file node, find internal symbol call edges
    let mut layers: Vec<(NodeIndex, String, Vec<NodeIndex>, Vec<petgraph::graph::EdgeIndex>)> =
        Vec::new();

    for &idx in &node_indices {
        let file_path = match &graph.graph[idx] {
            NodeKind::File(f) => &f.file_path,
            _ => continue,
        };

        // Find all symbol node indices whose file_path matches this file
        let file_symbols: HashSet<NodeIndex> = graph
            .graph
            .node_indices()
            .filter(|&ni| match &graph.graph[ni] {
                NodeKind::Symbol(s) => s.file_path == *file_path,
                _ => false,
            })
            .collect();

        if file_symbols.is_empty() {
            continue;
        }

        // Find call edges between symbols in this file
        let internal_edges: Vec<petgraph::graph::EdgeIndex> = graph
            .graph
            .edge_references()
            .filter(|e| {
                *e.weight() == EdgeKind::Calls
                    && file_symbols.contains(&e.source())
                    && file_symbols.contains(&e.target())
            })
            .map(|e| e.id())
            .collect();

        if internal_edges.is_empty() {
            continue;
        }

        // Collect the symbol nodes involved in those edges
        let mut involved_symbols: HashSet<NodeIndex> = HashSet::new();
        for &ei in &internal_edges {
            let (s, t) = graph.graph.edge_endpoints(ei).unwrap();
            involved_symbols.insert(s);
            involved_symbols.insert(t);
        }

        let mut sorted_symbols: Vec<_> = involved_symbols.into_iter().collect();
        sorted_symbols.sort_by_key(|n| n.index());

        let layer_id = sanitise_layer_id(idx, graph);
        layers.push((idx, layer_id, sorted_symbols, internal_edges));
    }

    let layer_file_ids: HashSet<NodeIndex> = layers.iter().map(|(idx, _, _, _)| *idx).collect();

    // Root board nodes
    for &idx in &node_indices {
        let has_layer = layer_file_ids.contains(&idx);
        match &graph.graph[idx] {
            NodeKind::File(file) => {
                if has_layer {
                    let layer_id = layers
                        .iter()
                        .find(|(i, _, _, _)| *i == idx)
                        .map(|(_, id, _, _)| id.as_str())
                        .unwrap();
                    writeln!(
                        output,
                        "node_{}: \"{}\" {{\n  link: layers.{}\n}}",
                        idx.index(),
                        file.file_path,
                        layer_id,
                    )
                    .unwrap();
                } else {
                    writeln!(output, "node_{}: \"{}\"", idx.index(), file.file_path).unwrap();
                }
            }
            NodeKind::Symbol(node) => {
                writeln!(
                    output,
                    "node_{}: \"{}\\n{}\\n{}:{}\"",
                    idx.index(),
                    node.name,
                    node.kind,
                    node.file_path,
                    node.line,
                )
                .unwrap();
            }
        }
    }

    // Root board edges
    for &edge_idx in &edge_indices {
        let (source, target) = graph.graph.edge_endpoints(edge_idx).unwrap();
        let weight = &graph.graph[edge_idx];
        writeln!(
            output,
            "node_{} -> node_{}: \"{}\"",
            source.index(),
            target.index(),
            weight,
        )
        .unwrap();
    }

    // Layers block
    if !layers.is_empty() {
        writeln!(output).unwrap();
        writeln!(output, "layers: {{").unwrap();

        for (_, layer_id, symbols, edges) in &layers {
            writeln!(output, "  {}: {{", layer_id).unwrap();

            for &sym_idx in symbols {
                if let NodeKind::Symbol(node) = &graph.graph[sym_idx] {
                    writeln!(
                        output,
                        "    node_{}: \"{}\\n{}\\n{}:{}\"",
                        sym_idx.index(),
                        node.name,
                        node.kind,
                        node.file_path,
                        node.line,
                    )
                    .unwrap();
                }
            }

            let mut sorted_edges = edges.clone();
            sorted_edges.sort_by_key(|e| e.index());

            for &edge_idx in &sorted_edges {
                let (source, target) = graph.graph.edge_endpoints(edge_idx).unwrap();
                let weight = &graph.graph[edge_idx];
                writeln!(
                    output,
                    "    node_{} -> node_{}: \"{}\"",
                    source.index(),
                    target.index(),
                    weight,
                )
                .unwrap();
            }

            writeln!(output, "  }}").unwrap();
        }

        writeln!(output, "}}").unwrap();
    }

    output.trim_end().to_string()
}

fn sanitise_layer_id(idx: NodeIndex, graph: &CodeGraph) -> String {
    let name = match &graph.graph[idx] {
        NodeKind::Symbol(s) => &s.name,
        NodeKind::File(f) => &f.file_path,
    };
    let sanitised: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    format!("{}_{}", sanitised, idx.index())
}

fn render_d2_node(
    output: &mut String,
    idx: NodeIndex,
    graph: &CodeGraph,
    indent: &str,
    has_layer: bool,
    layers: &[(NodeIndex, String, TraceResult)],
) {
    match &graph.graph[idx] {
        NodeKind::Symbol(node) => {
            let label = format!(
                "\"{}\\n{}\\n{}:{}\"",
                node.name, node.kind, node.file_path, node.line,
            );
            if has_layer {
                let layer_id = layers
                    .iter()
                    .find(|(i, _, _)| *i == idx)
                    .map(|(_, id, _)| id.as_str())
                    .unwrap();
                writeln!(
                    output,
                    "{}node_{}: {} {{\n{}  link: layers.{}\n{}}}",
                    indent,
                    idx.index(),
                    label,
                    indent,
                    layer_id,
                    indent,
                )
                .unwrap();
            } else {
                writeln!(output, "{}node_{}: {}", indent, idx.index(), label).unwrap();
            }
        }
        NodeKind::File(file) => {
            writeln!(
                output,
                "{}node_{}: \"{}\"",
                indent,
                idx.index(),
                file.file_path,
            )
            .unwrap();
        }
    }
}

pub fn render_mermaid(trace: &TraceResult, graph: &CodeGraph) -> String {
    let mut output = String::new();
    writeln!(output, "flowchart TD").unwrap();

    let mut node_indices: Vec<_> = trace.node_indices.iter().copied().collect();
    node_indices.sort_by_key(|n| n.index());

    for idx in &node_indices {
        match &graph.graph[*idx] {
            NodeKind::Symbol(node) => {
                writeln!(
                    output,
                    "    node_{}[\"{}<br/>{}<br/>{}:{}\"]",
                    idx.index(),
                    node.name,
                    node.kind,
                    node.file_path,
                    node.line,
                )
                .unwrap();
            }
            NodeKind::File(file) => {
                writeln!(
                    output,
                    "    node_{}[\"{}\"]",
                    idx.index(),
                    file.file_path,
                )
                .unwrap();
            }
        }
    }

    let mut edge_indices: Vec<_> = trace.edge_indices.iter().copied().collect();
    edge_indices.sort_by_key(|e| e.index());

    for edge_idx in &edge_indices {
        let (source, target) = graph.graph.edge_endpoints(*edge_idx).unwrap();
        let weight = &graph.graph[*edge_idx];
        writeln!(
            output,
            "    node_{} -->|\"{}\"| node_{}",
            source.index(),
            weight,
            target.index(),
        )
        .unwrap();
    }

    output.trim_end().to_string()
}

pub fn write_output(content: &str, output: Option<&Path>) -> io::Result<()> {
    match output {
        Some(path) => std::fs::write(path, content),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}
