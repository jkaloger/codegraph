---
title: Symbol trace
type: iteration
status: accepted
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/stories/STORY-002-symbol-trace.md
---



## Test Plan

- **AC-1 (default trace):** Index a fixture project, run `codegraph trace MyFunction`. Assert stdout is valid d2, contains the target node and neighbours within depth 2 in both directions.
- **AC-2 (--depth):** Run with `--depth 3`. Assert nodes at depth 3 appear; nodes at depth 4 do not.
- **AC-3 (--direction in):** Run with `--direction in`. Assert only caller nodes appear, no callees.
- **AC-4 (--direction out):** Run with `--direction out`. Assert only callee nodes appear, no callers.
- **AC-5 (--kind call):** Add ref edges to fixture. Run with `--kind call`. Assert ref edges absent from output.
- **AC-6 (--kind all):** Run with `--kind all`. Assert call and ref edges both present.
- **AC-7 (--format mermaid):** Run with `--format mermaid`. Assert output starts with `flowchart` and parses as valid mermaid.
- **AC-8 (--output file):** Run with `--output graph.d2`. Assert file written, stdout empty.
- **AC-9 (missing symbol):** Run with a nonexistent symbol. Assert exit code != 0 and stderr contains error.
- **AC-10 (ambiguous symbol):** Add two symbols with the same name in different files. Assert CLI lists both with file:line and exits non-zero.
- **AC-11 (node labels):** Parse d2 output nodes. Assert labels contain symbol name, kind, file path, and line number.
- **AC-12 (edge labels):** Parse d2 output edges. Assert labels contain edge kind and source line.

## Changes

### Task 1: Bounded graph traversal

**ACs addressed:** AC-1, AC-2, AC-3, AC-4, AC-5, AC-6

**Files:**
- Create: `src/traverse.rs`
- Test: `tests/traverse_test.rs`

**What to implement:**
A `trace` function that takes the petgraph `CodeGraph`, a starting `NodeIndex`, a max depth, a direction enum (`In | Out | Both`), and an edge-kind filter set. Performs BFS from the start node, respecting direction and depth constraints. Returns a subgraph (set of node indices and edge indices) representing the trace result. Skip edges whose kind is not in the filter set.

**How to verify:**
Build a small in-memory graph in tests. Assert correct node/edge sets for each combination of depth, direction, and kind filter. `cargo test traverse`

---

### Task 2: D2 renderer

**ACs addressed:** AC-1, AC-8, AC-11, AC-12

**Files:**
- Create: `src/render.rs`
- Test: `tests/render_test.rs`

**What to implement:**
A `render_d2` function that takes the trace subgraph and the full `CodeGraph`, and produces a d2 string. Node labels must include symbol name, kind, file path, and line number. Edge labels must include edge kind and source line. A `write_output` helper that either prints to stdout or writes to a file path.

**How to verify:**
Pass a known subgraph, snapshot the d2 output. Assert node/edge label content. `cargo test render`

---

### Task 3: Mermaid renderer

**ACs addressed:** AC-7

**Files:**
- Modify: `src/render.rs`
- Test: `tests/render_test.rs`

**What to implement:**
A `render_mermaid` function with the same interface as `render_d2`. Produces a `flowchart TD` block with the same label requirements. Add a `Format` enum (`D2 | Mermaid`) and a dispatcher function.

**How to verify:**
Snapshot test the mermaid output. Assert it starts with `flowchart TD`. `cargo test render`

---

### Task 4: Trace CLI subcommand

**ACs addressed:** AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-10

**Files:**
- Modify: `src/main.rs`
- Test: `tests/cli_trace_test.rs`

**What to implement:**
Add a `trace` subcommand to clap with args: `<symbol>` (positional), `--depth` (default 2), `--direction` (in/out/both, default both), `--kind` (call/all, default call), `--format` (d2/mermaid, default d2), `--output` (optional path). The handler loads the index, resolves the symbol name to a node (handling missing and ambiguous cases), calls `trace`, then renders and outputs.

For AC-9: if no node matches, print error to stderr and exit 1.
For AC-10: if multiple nodes match, list them with `file:line` and exit 1.

**How to verify:**
Integration tests using `assert_cmd` against fixture projects. `cargo test cli_trace`

## Notes

This iteration depends on ITER-001 providing: `CodeGraph` (petgraph), `SymbolNode`, `EdgeKind`, the `index` command, and serialized graph storage. The traversal and rendering modules are fully new.
