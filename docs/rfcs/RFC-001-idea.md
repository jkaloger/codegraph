---
title: "codegraph: symbol & call graph explorer"
type: rfc
status: accepted
author: "unknown"
date: 2026-03-17
tags: [rust, tree-sitter, developer-tools]
---


## Problem

Developers exploring unfamiliar codebases repeatedly ask the same questions: "Where is this defined?", "Who calls this?", "What does this function touch?". Existing tools (IDE go-to-definition, grep) answer these one hop at a time. There's no quick way to get a visual neighbourhood of a symbol showing its definition, callers, and callees at a configurable depth.

## Intent

Build `codegraph`, a Rust CLI that parses TypeScript/JS source using tree-sitter, builds an in-memory symbol and call graph, and renders static diagrams (d2 or mermaid) for a queried symbol's neighbourhood.

A developer should be able to run something like:

```
codegraph trace processOrder --depth 2 --format d2
```

and get a diagram showing `processOrder`, everything it calls (2 levels deep), and everything that calls it (2 levels up).

## Design

### Core Pipeline

```
source files -> parse (tree-sitter) -> extract (symbols + edges) -> graph (petgraph) -> filter -> render (d2/mermaid)
```

Each stage is a distinct module with a clear boundary:

1. **Parse**: Walk the project directory, parse each `.ts`/`.js`/`.tsx`/`.jsx` file with tree-sitter-typescript. Produce concrete syntax trees.

2. **Extract**: Visit each CST to extract:
   - Symbol definitions (functions, classes, methods, variables, type aliases)
   - Call expressions (function calls, method calls)
   - Import/export relationships (module edges)
   - Symbol references (where a symbol is read or written)

3. **Graph**: Store extracted data in a `petgraph::DiGraph` where:
   - Nodes are symbols (keyed by file path + symbol name + kind)
   - Edges are relationships: `calls`, `called_by`, `imports`, `exports`, `references`

4. **Filter**: Given a query symbol, perform bounded BFS/DFS traversal:
   - `--depth N` controls how many hops outward
   - `--direction in|out|both` controls edge direction
   - `--kind call|ref|import|all` filters edge types

5. **Render**: Convert the filtered subgraph to d2 or mermaid syntax. Each node includes file location. Edges are labelled with relationship type.

### Interface Sketches

```
@draft SymbolNode {
    id: NodeId,
    name: String,
    kind: SymbolKind,       // Function, Class, Method, Variable, TypeAlias
    file_path: PathBuf,
    line: usize,
    module: String,         // resolved module path
}

@draft Edge {
    kind: EdgeKind,         // Calls, CalledBy, Imports, References
    source_line: usize,     // where the relationship originates
}

@draft GraphQuery {
    symbol: String,         // symbol name or file:symbol
    depth: usize,           // max hops (default: 2)
    direction: Direction,   // In, Out, Both
    edge_filter: EdgeKind,  // which edge types to traverse
}
```

### CLI Surface

```
codegraph index [path]              # build/rebuild the graph for a project
codegraph trace <symbol> [options]  # render neighbourhood graph
codegraph list [--kind <kind>]      # list known symbols
```

Options for `trace`:
- `--depth <N>` (default: 2)
- `--direction <in|out|both>` (default: both)
- `--format <d2|mermaid>` (default: d2)
- `--kind <call|ref|import|all>` (default: call)
- `--output <file>` (default: stdout)

### Key Decisions

**Why tree-sitter over tsc/LSP?** Tree-sitter is fast, incremental, and doesn't require a valid project setup (no `tsconfig.json` needed). It trades type-level precision for speed and zero-config. For call graph and symbol reference analysis, syntactic accuracy is sufficient for the MVP.

**Why petgraph?** It's the standard Rust graph library. Supports directed graphs, BFS/DFS traversal, subgraph extraction, and serialization. No reason to build a custom graph structure.

**Why d2 as default output?** d2 supports board linking (clicking a node could open a deeper subgraph), auto-layout, and renders to SVG. Mermaid is offered as an alternative for GitHub/Markdown embedding.

### Limitations (MVP)

- TypeScript/JS only. Language support is pluggable (swap the tree-sitter grammar and extractor) but the MVP targets one ecosystem.
- No type-level resolution. If `foo()` is called on a variable, we resolve it by name, not by type. This means some edges may be ambiguous (e.g., two functions named `handle` in different modules).
- No incremental updates. `codegraph index` rebuilds the full graph. Incremental indexing is a future optimisation.
- Dynamic dispatch (callbacks, higher-order functions) won't be traced.

## Stories

1. **Project indexing**: Parse a TS/JS project with tree-sitter, extract symbols and call edges, store in a petgraph. CLI: `codegraph index <path>`.

2. **Symbol trace**: Given an indexed project and a symbol name, perform bounded traversal and output a d2/mermaid diagram. CLI: `codegraph trace <symbol>`.

3. **Module dependency graph**: Extract import/export edges and render a file-level dependency diagram. Reuses the same graph infrastructure with a different edge filter.

4. **Symbol reference graph**: Add reference edges (read/write sites) alongside call edges. Extends the extractor and trace output.
