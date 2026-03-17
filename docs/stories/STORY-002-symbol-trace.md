---
title: Symbol trace
type: story
status: accepted
author: unknown
date: 2026-03-17
tags: []
related:
- implements: docs/rfcs/RFC-001-idea.md
---



## Context

Story 1 builds the parsed graph from TypeScript/JS source. This story covers the trace query: given a symbol name, traverse the graph and render a diagram showing the call/reference relationships up to a bounded depth.

## Acceptance Criteria

- **Given** an indexed project with call edges
  **When** the user runs `codegraph trace MyFunction`
  **Then** a d2 diagram is printed to stdout showing nodes reachable within depth 2 (default) in both directions

- **Given** a `--depth 3` flag
  **When** the trace runs
  **Then** traversal stops at depth 3, and no deeper nodes appear in output

- **Given** `--direction in`
  **When** the trace runs
  **Then** only callers (incoming edges) of the target symbol are included

- **Given** `--direction out`
  **When** the trace runs
  **Then** only callees (outgoing edges) of the target symbol are included

- **Given** `--kind call`
  **When** the trace runs
  **Then** only call edges are traversed; ref and import edges are excluded

- **Given** `--kind all`
  **When** the trace runs
  **Then** call, ref, and import edges are all traversed

- **Given** `--format mermaid`
  **When** the trace runs
  **Then** the output is a valid mermaid flowchart instead of d2

- **Given** `--output graph.d2`
  **When** the trace runs
  **Then** the diagram is written to `graph.d2` instead of stdout

- **Given** a symbol name that does not exist in the graph
  **When** the trace runs
  **Then** the CLI exits with a non-zero code and prints an error message to stderr

- **Given** a symbol name that matches multiple nodes (ambiguous)
  **When** the trace runs
  **Then** the CLI lists the matching symbols with file locations and asks the user to disambiguate

- **Given** rendered d2 output
  **When** inspecting a node label
  **Then** it contains the symbol name, kind, file path, and line number

- **Given** rendered output with edges
  **When** inspecting an edge label
  **Then** it contains the edge kind and source line

## Scope

### In Scope

- `codegraph trace <symbol>` CLI command
- Options: `--depth N`, `--direction in|out|both`, `--format d2|mermaid`, `--kind call|ref|import|all`, `--output file`
- Bounded BFS/DFS traversal of petgraph given a GraphQuery
- Depth limiting, direction filtering, edge kind filtering
- d2 output rendering (nodes with file location, labelled edges)
- Mermaid output rendering
- File output via `--output` flag, stdout as default
- Error handling for symbol-not-found and ambiguous matches

### Out of Scope

- Graph construction and indexing (Story 1)
- Import/export edge extraction (Story 3)
- Reference edge extraction (Story 4)
- `codegraph index` and `codegraph list` commands (Story 1)
