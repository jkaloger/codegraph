---
title: Project indexing
type: story
status: draft
author: unknown
date: 2026-03-17
tags: []
related:
- implements: docs/rfcs/RFC-001-idea.md
---


## Context

`codegraph` needs to walk a project directory, parse TypeScript/JavaScript source files into concrete syntax trees, extract symbol definitions and call expressions, and store them in a directed graph. This story covers the foundational indexing pipeline and the two CLI commands that expose it: `codegraph index` and `codegraph list`.

## Acceptance Criteria

- **Given** a directory containing `.ts`, `.js`, `.tsx`, and `.jsx` files (including nested subdirectories)
  **When** I run `codegraph index <path>`
  **Then** all matching source files are discovered and parsed via tree-sitter, and the process completes without error

- **Given** source files containing function declarations, class declarations, method definitions, variable declarations, and type alias declarations
  **When** the indexer parses these files
  **Then** a `SymbolNode` is created for each definition with correct `name`, `kind`, `file_path`, `line`, and `module` fields

- **Given** source files containing function calls and method calls
  **When** the indexer parses these files
  **Then** `Calls` edges are created between the caller symbol and the callee symbol in the graph

- **Given** an indexed project
  **When** I run `codegraph list`
  **Then** all known symbols are printed, one per line, with their kind and file location

- **Given** an indexed project with mixed symbol kinds
  **When** I run `codegraph list --kind Function`
  **Then** only symbols of kind `Function` are printed

- **Given** a directory with no matching source files
  **When** I run `codegraph index <path>`
  **Then** the command completes successfully with zero symbols in the graph and an informational message

- **Given** a source file that tree-sitter fails to fully parse (e.g. syntax errors)
  **When** the indexer processes that file
  **Then** partial results are extracted where possible and a warning is emitted, without aborting the entire index run

## Scope

### In Scope

- Rust project scaffolding: `Cargo.toml`, module structure (`cli`, `walk`, `parse`, `extract`, `graph`)
- Directory walking to discover `.ts`, `.js`, `.tsx`, `.jsx` files (respecting common ignore patterns like `node_modules`)
- Tree-sitter parsing of each discovered file to produce CSTs
- Extraction of symbol definitions: functions, classes, methods, variables, type aliases
- Extraction of call expressions: function calls, method calls
- Storage in `petgraph::DiGraph` with `SymbolNode` as node weight and `Edge` (kind: `Calls`/`CalledBy`) as edge weight
- `codegraph index <path>` CLI command
- `codegraph list [--kind <kind>]` CLI command

### Out of Scope

- Graph traversal and filtering beyond simple kind filtering (Story 2)
- d2/mermaid rendering (Story 2)
- `codegraph trace` command (Story 2)
- Import/export edge extraction (Story 3)
- Reference edge extraction (Story 4)
