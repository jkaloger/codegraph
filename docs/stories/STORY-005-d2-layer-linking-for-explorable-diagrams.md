---
title: D2 layer linking for explorable diagrams
type: story
status: accepted
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/rfcs/RFC-001-idea.md
---



## Context

The RFC chose d2 over mermaid partly because d2 supports board linking -- clicking a node can open a deeper subgraph. The current `trace` and `render` commands produce flat d2 output where every node is a terminal label. For large codebases, a single flat diagram either gets too crowded (high depth) or too shallow (low depth) to be useful.

With d2 layers, each symbol node in a trace becomes a clickable entry point into its own neighbourhood. The user starts with a high-level overview and drills into the symbols they care about, exploring the codebase spatially rather than re-running commands.

## Acceptance Criteria

- **Given** a project is indexed and a symbol exists with callers and callees
  **When** the user runs `codegraph trace <symbol> --format d2 --layers`
  **Then** the root board shows the symbol's neighbourhood at the requested depth, and each non-leaf symbol node links to a nested board containing that symbol's own depth-1 trace

- **Given** a trace is rendered with `--layers`
  **When** the d2 output is compiled with `d2`
  **Then** each linked node navigates to its nested board on click, and a back link returns to the parent board

- **Given** a trace is rendered with `--layers` and a node's depth-1 neighbourhood contains only the edges already visible on the parent board
  **When** the output is generated
  **Then** that node does not get a nested board (no empty/redundant layers)

- **Given** a project is indexed
  **When** the user runs `codegraph render --format d2 --layers`
  **Then** each file node links to a nested board showing that file's internal symbol-level call graph

- **Given** the user runs a trace or render without `--layers`
  **When** the output is generated
  **Then** the behaviour is unchanged from the current flat output (backwards compatible)

## Scope

### In Scope

- `--layers` flag on `trace` and `render` commands (d2 format only)
- Nested d2 boards generated per non-leaf node
- Back-links from child boards to parent
- Redundant layer suppression (skip layers that add no new information)

### Out of Scope

- Mermaid layer support (mermaid has no equivalent of d2 boards)
- Interactive web viewer or server mode
- Incremental/cached layer generation
- Custom depth per layer (all nested boards use depth 1)
