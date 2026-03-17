use std::fmt::Write;
use std::io;
use std::path::Path;
use std::str::FromStr;

use crate::graph::{CodeGraph, NodeKind};
use crate::traverse::TraceResult;

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
            "node_{} -> node_{}: \"{:?}\"",
            source.index(),
            target.index(),
            weight,
        )
        .unwrap();
    }

    output.trim_end().to_string()
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
            "    node_{} -->|\"{:?}\"| node_{}",
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
