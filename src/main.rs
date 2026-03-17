use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use codegraph::extract::{self, RefKind};
use codegraph::graph::{CodeGraph, EdgeKind, NodeKind, SymbolKind};
use codegraph::parse;
use codegraph::render::{self, Format};
use codegraph::resolve::{self, ResolveResult};
use codegraph::traverse::{self, Direction};
use codegraph::walk;

#[derive(Parser)]
#[command(name = "codegraph")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a directory and print a summary
    Index {
        /// Path to the directory to index
        path: PathBuf,
    },
    /// Index a directory and list all symbols
    List {
        /// Path to the directory to index
        path: PathBuf,
        /// Filter symbols by kind (Function, Class, Method, Variable, TypeAlias)
        #[arg(long)]
        kind: Option<SymbolKind>,
    },
    /// Trace a symbol's call graph
    Trace {
        /// Symbol name to trace
        symbol: String,
        /// Path to the directory to index
        path: PathBuf,
        /// Max traversal depth
        #[arg(long, default_value = "2")]
        depth: usize,
        /// Traversal direction: in, out, both
        #[arg(long, default_value = "both")]
        direction: Direction,
        /// Edge kind filter: call, ref, import, all
        #[arg(long, default_value = "call")]
        kind: String,
        /// Output format: d2, mermaid
        #[arg(long, default_value = "d2")]
        format: Format,
        /// Output file path (prints to stdout if omitted)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Render a file-level dependency diagram
    Render {
        /// Path to the directory to index
        path: PathBuf,
        /// Edge kind filter: import, all
        #[arg(long, default_value = "import")]
        kind: String,
        /// Output format: d2, mermaid
        #[arg(long, default_value = "d2")]
        format: Format,
        /// Output file path (prints to stdout if omitted)
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn build_graph(path: &PathBuf) -> Result<CodeGraph> {
    let files = walk::discover_files(path)?;

    if files.is_empty() {
        println!("No matching source files found in {}", path.display());
        return Ok(CodeGraph::new());
    }

    let mut graph = CodeGraph::new();
    let mut symbol_indices: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for file in &files {
        let source = std::fs::read_to_string(file)?;
        let tree = parse::parse_file(file)?;
        let result = extract::extract(&tree, &source, file);

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
            let Some(scope) = &reference.enclosing_scope else {
                continue;
            };
            let Some(&from) = symbol_indices.get(scope) else {
                continue;
            };
            let Some(&to) = symbol_indices.get(&reference.symbol_name) else {
                continue;
            };
            let edge_kind = match reference.kind {
                RefKind::Read => EdgeKind::ReadsFrom,
                RefKind::Write => EdgeKind::WritesTo,
            };
            graph.add_reference(from, to, edge_kind);
        }

        let src_path = file.canonicalize().unwrap_or_else(|_| file.clone());
        let src_path_str = src_path.to_string_lossy().to_string();

        for specifier in &result.imports {
            if let ResolveResult::Resolved(target) = resolve::resolve_specifier(specifier, file) {
                let target_str = target.to_string_lossy().to_string();
                let src_idx = graph.add_file(&src_path_str);
                let tgt_idx = graph.add_file(&target_str);
                graph.add_import(src_idx, tgt_idx);
            }
        }

        for specifier in &result.reexports {
            if let ResolveResult::Resolved(target) = resolve::resolve_specifier(specifier, file) {
                let target_str = target.to_string_lossy().to_string();
                let src_idx = graph.add_file(&src_path_str);
                let tgt_idx = graph.add_file(&target_str);
                graph.add_export(src_idx, tgt_idx);
            }
        }
    }

    Ok(graph)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index { path } => {
            let graph = build_graph(&path)?;
            let symbol_count = graph.symbols().count();
            let call_count = graph.graph.edge_references()
                .filter(|e| *e.weight() == EdgeKind::Calls)
                .count();
            let import_count = graph.graph.edge_references()
                .filter(|e| *e.weight() == EdgeKind::Imports)
                .count();
            let export_count = graph.graph.edge_references()
                .filter(|e| *e.weight() == EdgeKind::Exports)
                .count();
            let reference_count = graph.graph.edge_references()
                .filter(|e| matches!(e.weight(), EdgeKind::ReadsFrom | EdgeKind::WritesTo))
                .count();
            println!(
                "Indexed {} symbols, {} calls, {} imports, {} exports, {} references",
                symbol_count, call_count, import_count, export_count, reference_count
            );
        }
        Commands::List { path, kind } => {
            let graph = build_graph(&path)?;

            for symbol in graph.symbols() {
                if let Some(ref filter_kind) = kind {
                    if &symbol.kind != filter_kind {
                        continue;
                    }
                }
                println!("{}\t{}\t{}:{}", symbol.kind, symbol.name, symbol.file_path, symbol.line);
            }
        }
        Commands::Trace {
            symbol,
            path,
            depth,
            direction,
            kind,
            format,
            output,
        } => {
            let graph = build_graph(&path)?;

            let edge_filter: HashSet<EdgeKind> = match kind.as_str() {
                "call" => HashSet::from([EdgeKind::Calls]),
                "ref" => HashSet::from([EdgeKind::ReadsFrom, EdgeKind::WritesTo]),
                "import" => HashSet::from([EdgeKind::Imports, EdgeKind::Exports]),
                "all" => HashSet::from([
                    EdgeKind::Calls,
                    EdgeKind::Imports,
                    EdgeKind::Exports,
                    EdgeKind::ReadsFrom,
                    EdgeKind::WritesTo,
                ]),
                other => {
                    eprintln!("Unknown kind: {other}");
                    std::process::exit(1);
                }
            };

            let start = if kind == "import" {
                find_file_node_for_symbol(&graph, &symbol)?
            } else {
                find_symbol_node(&graph, &symbol)?
            };

            let trace_result = traverse::trace(&graph, start, depth, &direction, &edge_filter);
            let content = render::render(&trace_result, &graph, format);
            render::write_output(&content, output.as_deref())?;
        }
        Commands::Render {
            path,
            kind,
            format,
            output,
        } => {
            let graph = build_graph(&path)?;

            let edge_filter: HashSet<EdgeKind> = match kind.as_str() {
                "import" => HashSet::from([EdgeKind::Imports, EdgeKind::Exports]),
                "all" => HashSet::from([EdgeKind::Calls, EdgeKind::Imports, EdgeKind::Exports]),
                other => {
                    eprintln!("Unknown kind: {other}");
                    std::process::exit(1);
                }
            };

            let trace_result = collect_edges_by_kind(&graph, &edge_filter);
            let content = render::render(&trace_result, &graph, format);
            render::write_output(&content, output.as_deref())?;
        }
    }

    Ok(())
}

fn find_symbol_node(graph: &CodeGraph, symbol: &str) -> Result<petgraph::graph::NodeIndex> {
    let matches = graph.find_symbols_by_name(symbol);

    if matches.is_empty() {
        eprintln!("Symbol '{symbol}' not found");
        std::process::exit(1);
    }

    if matches.len() > 1 {
        eprintln!("Ambiguous symbol '{symbol}', multiple matches:");
        for idx in &matches {
            if let NodeKind::Symbol(node) = &graph.graph[*idx] {
                eprintln!("  {} {}:{}", node.name, node.file_path, node.line);
            }
        }
        std::process::exit(1);
    }

    Ok(matches[0])
}

fn find_file_node_for_symbol(graph: &CodeGraph, symbol: &str) -> Result<petgraph::graph::NodeIndex> {
    let matches = graph.find_symbols_by_name(symbol);

    if matches.is_empty() {
        eprintln!("Symbol '{symbol}' not found");
        std::process::exit(1);
    }

    if matches.len() > 1 {
        eprintln!("Ambiguous symbol '{symbol}', multiple matches:");
        for idx in &matches {
            if let NodeKind::Symbol(node) = &graph.graph[*idx] {
                eprintln!("  {} {}:{}", node.name, node.file_path, node.line);
            }
        }
        std::process::exit(1);
    }

    let file_path = match &graph.graph[matches[0]] {
        NodeKind::Symbol(s) => s.file_path.clone(),
        _ => unreachable!(),
    };

    let canonical = std::path::Path::new(&file_path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(&file_path))
        .to_string_lossy()
        .to_string();

    match graph.find_file_node(&canonical) {
        Some(idx) => Ok(idx),
        None => {
            eprintln!("No file node found for '{file_path}'");
            std::process::exit(1);
        }
    }
}

fn collect_edges_by_kind(
    graph: &CodeGraph,
    edge_filter: &HashSet<EdgeKind>,
) -> traverse::TraceResult {
    use petgraph::visit::EdgeRef;

    let mut node_indices = HashSet::new();
    let mut edge_indices = HashSet::new();

    for edge in graph.graph.edge_references() {
        if !edge_filter.contains(edge.weight()) {
            continue;
        }
        edge_indices.insert(edge.id());
        node_indices.insert(edge.source());
        node_indices.insert(edge.target());
    }

    traverse::TraceResult {
        node_indices,
        edge_indices,
    }
}
