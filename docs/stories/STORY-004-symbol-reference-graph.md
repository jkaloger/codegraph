---
title: Symbol reference graph
type: story
status: accepted
author: unknown
date: 2026-03-17
tags: []
related:
- implements: docs/rfcs/RFC-001-idea.md
---



## Context

`codegraph` builds a symbol and call graph from TypeScript/JS source via tree-sitter. Stories 1 and 2 cover symbol/call extraction and traversal/rendering. This story adds symbol reference edges (reads and writes) to the graph, enabling users to trace where symbols are used beyond just call sites.

## Acceptance Criteria

- **Given** a TypeScript/JS source file containing variable reads and writes
  **When** the extractor visits the tree-sitter CST
  **Then** reference sites are identified and classified as read or write

- **Given** a source file with `x = foo` and `bar(x)`
  **When** extraction completes
  **Then** `x` has a write-reference edge from the assignment and a read-reference edge from the call argument

- **Given** extracted reference edges stored in petgraph
  **When** the user runs `codegraph trace <symbol> --kind ref`
  **Then** only reference edges (reads and writes) appear in the output

- **Given** extracted reference and call edges in petgraph
  **When** the user runs `codegraph trace <symbol> --kind all`
  **Then** both call edges and reference edges appear in the trace diagram

- **Given** a symbol that is only referenced but never called
  **When** the user runs `codegraph trace <symbol> --kind call`
  **Then** no edges are returned for that symbol

- **Given** a destructuring assignment like `const { a, b } = obj`
  **When** extraction completes
  **Then** `a` and `b` each have write-reference edges from the destructuring site

## Scope

### In Scope

- Extraction of symbol reference sites (reads and writes) from tree-sitter CSTs
- Distinguishing read vs write references where possible
- Adding reference edges (`references` / `referenced_by`) to the existing petgraph
- Filtering trace output by `--kind ref`
- Extending trace diagrams to show reference edges alongside call edges when `--kind all`

### Out of Scope

- Symbol definition extraction (Story 1)
- Call edge extraction (Story 1)
- Import/export edges (Story 3)
- Traversal and rendering infrastructure (Story 2 reuses existing)
- Type-level reference resolution (MVP limitation)
