use std::collections::{HashSet, VecDeque};
use std::str::FromStr;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::graph::{CodeGraph, EdgeKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
    Both,
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "in" => Ok(Direction::In),
            "out" => Ok(Direction::Out),
            "both" => Ok(Direction::Both),
            _ => Err(format!("unknown direction: {s}")),
        }
    }
}

#[derive(Debug)]
pub struct TraceResult {
    pub node_indices: HashSet<NodeIndex>,
    pub edge_indices: HashSet<EdgeIndex>,
}

pub fn trace(
    graph: &CodeGraph,
    start: NodeIndex,
    depth: usize,
    direction: &Direction,
    edge_filter: &HashSet<EdgeKind>,
) -> TraceResult {
    let mut visited_nodes: HashSet<NodeIndex> = HashSet::new();
    let mut result_edges: HashSet<EdgeIndex> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();

    visited_nodes.insert(start);
    queue.push_back((start, 0));

    while let Some((node, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }

        let directions: Vec<petgraph::Direction> = match direction {
            Direction::Out => vec![petgraph::Direction::Outgoing],
            Direction::In => vec![petgraph::Direction::Incoming],
            Direction::Both => vec![petgraph::Direction::Outgoing, petgraph::Direction::Incoming],
        };

        for dir in directions {
            for edge in graph.graph.edges_directed(node, dir) {
                if !edge_filter.contains(edge.weight()) {
                    continue;
                }

                let neighbor = match dir {
                    petgraph::Direction::Outgoing => edge.target(),
                    petgraph::Direction::Incoming => edge.source(),
                };

                result_edges.insert(edge.id());

                if visited_nodes.insert(neighbor) {
                    queue.push_back((neighbor, current_depth + 1));
                }
            }
        }
    }

    TraceResult {
        node_indices: visited_nodes,
        edge_indices: result_edges,
    }
}
