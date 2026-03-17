use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Clone, Debug, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Method,
    Variable,
    TypeAlias,
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolKind::Function => write!(f, "Function"),
            SymbolKind::Class => write!(f, "Class"),
            SymbolKind::Method => write!(f, "Method"),
            SymbolKind::Variable => write!(f, "Variable"),
            SymbolKind::TypeAlias => write!(f, "TypeAlias"),
        }
    }
}

impl FromStr for SymbolKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Function" => Ok(SymbolKind::Function),
            "Class" => Ok(SymbolKind::Class),
            "Method" => Ok(SymbolKind::Method),
            "Variable" => Ok(SymbolKind::Variable),
            "TypeAlias" => Ok(SymbolKind::TypeAlias),
            _ => Err(format!("unknown symbol kind: {s}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SymbolNode {
    pub name: String,
    pub kind: SymbolKind,
    pub file_path: String,
    pub line: usize,
    pub module: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FileNode {
    pub file_path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Symbol(SymbolNode),
    File(FileNode),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EdgeKind {
    Calls,
    Imports,
    Exports,
}

pub struct CodeGraph {
    pub graph: DiGraph<NodeKind, EdgeKind>,
    file_index: HashMap<String, NodeIndex>,
}

impl CodeGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            file_index: HashMap::new(),
        }
    }

    pub fn add_symbol(&mut self, symbol: SymbolNode) -> NodeIndex {
        self.graph.add_node(NodeKind::Symbol(symbol))
    }

    pub fn add_file(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.file_index.get(path) {
            return idx;
        }
        let idx = self.graph.add_node(NodeKind::File(FileNode {
            file_path: path.to_string(),
        }));
        self.file_index.insert(path.to_string(), idx);
        idx
    }

    pub fn add_call(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, EdgeKind::Calls);
    }

    pub fn add_import(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, EdgeKind::Imports);
    }

    pub fn add_export(&mut self, from: NodeIndex, to: NodeIndex) {
        self.graph.add_edge(from, to, EdgeKind::Exports);
    }

    pub fn symbols(&self) -> impl Iterator<Item = &SymbolNode> {
        self.graph.node_weights().filter_map(|n| match n {
            NodeKind::Symbol(s) => Some(s),
            _ => None,
        })
    }

    pub fn find_symbols_by_name(&self, name: &str) -> Vec<NodeIndex> {
        self.graph
            .node_indices()
            .filter(|&idx| matches!(&self.graph[idx], NodeKind::Symbol(s) if s.name == name))
            .collect()
    }

    pub fn find_file_node(&self, path: &str) -> Option<NodeIndex> {
        self.file_index.get(path).copied()
    }

    pub fn file_nodes(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.graph.node_indices().filter(|&idx| matches!(&self.graph[idx], NodeKind::File(_)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_symbols_and_call_edge() {
        let mut cg = CodeGraph::new();

        let a = cg.add_symbol(SymbolNode {
            name: "foo".into(),
            kind: SymbolKind::Function,
            file_path: "src/index.ts".into(),
            line: 1,
            module: "index".into(),
        });

        let b = cg.add_symbol(SymbolNode {
            name: "bar".into(),
            kind: SymbolKind::Function,
            file_path: "src/index.ts".into(),
            line: 10,
            module: "index".into(),
        });

        cg.add_call(a, b);

        assert_eq!(cg.graph.node_count(), 2);
        assert_eq!(cg.graph.edge_count(), 1);
    }

    #[test]
    fn test_add_file_deduplicates() {
        let mut cg = CodeGraph::new();
        let f1 = cg.add_file("src/index.ts");
        let f2 = cg.add_file("src/utils.ts");
        let f1_again = cg.add_file("src/index.ts");

        assert_eq!(f1, f1_again);
        assert_ne!(f1, f2);
        assert_eq!(cg.graph.node_count(), 2);
    }

    #[test]
    fn test_import_and_export_edges() {
        let mut cg = CodeGraph::new();
        let f1 = cg.add_file("src/index.ts");
        let f2 = cg.add_file("src/utils.ts");

        cg.add_import(f1, f2);
        cg.add_export(f2, f1);

        assert_eq!(cg.graph.node_count(), 2);
        assert_eq!(cg.graph.edge_count(), 2);

        let edges: Vec<_> = cg
            .graph
            .edge_indices()
            .map(|e| cg.graph[e].clone())
            .collect();
        assert!(edges.contains(&EdgeKind::Imports));
        assert!(edges.contains(&EdgeKind::Exports));
    }

    #[test]
    fn test_symbols_filters_file_nodes() {
        let mut cg = CodeGraph::new();
        cg.add_symbol(SymbolNode {
            name: "foo".into(),
            kind: SymbolKind::Function,
            file_path: "src/index.ts".into(),
            line: 1,
            module: "index".into(),
        });
        cg.add_file("src/index.ts");

        assert_eq!(cg.graph.node_count(), 2);
        assert_eq!(cg.symbols().count(), 1);
    }

    #[test]
    fn test_find_symbols_by_name_ignores_file_nodes() {
        let mut cg = CodeGraph::new();
        cg.add_symbol(SymbolNode {
            name: "foo".into(),
            kind: SymbolKind::Function,
            file_path: "src/index.ts".into(),
            line: 1,
            module: "index".into(),
        });
        cg.add_file("foo");

        let matches = cg.find_symbols_by_name("foo");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_mixed_node_and_edge_kinds() {
        let mut cg = CodeGraph::new();
        let f1 = cg.add_file("src/index.ts");
        let f2 = cg.add_file("src/utils.ts");
        let sym = cg.add_symbol(SymbolNode {
            name: "greet".into(),
            kind: SymbolKind::Function,
            file_path: "src/index.ts".into(),
            line: 1,
            module: "index".into(),
        });

        cg.add_import(f1, f2);
        cg.add_call(sym, sym);

        assert_eq!(cg.graph.node_count(), 3);
        assert_eq!(cg.graph.edge_count(), 2);
    }
}
