---
title: D2 layer linking for explorable diagrams
type: iteration
status: accepted
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/stories/STORY-005-d2-layer-linking-for-explorable-diagrams.md
---



## Changes

### Task 1: Add `--layers` flag to Trace and Render CLI commands

**ACs addressed:** AC-5 (backwards compatibility)

**Files:**
- Modify: `src/main.rs`

**What to implement:**
Add a `layers: bool` flag (`--layers`) to both the `Trace` and `Render` variants of the `Commands` enum (around lines 38-72). Default is `false`. When `false`, the existing codepath is unchanged. When `true` and format is mermaid, exit with an error message: "layers are only supported with d2 format". Pass the flag through to the render call in Task 3.

**How to verify:**
```sh
cargo test
codegraph trace dispatch ./path --format mermaid --layers  # should error
codegraph trace dispatch ./path                             # unchanged output
```

---

### Task 2: Implement `render_d2_layers` for trace output

**ACs addressed:** AC-1 (nested boards per non-leaf symbol), AC-2 (back links), AC-3 (redundant layer suppression)

**Files:**
- Modify: `src/render.rs`
- Modify: `src/traverse.rs` (expose `trace` for reuse)

**What to implement:**

Add a new function `render_d2_layers(trace: &TraceResult, graph: &CodeGraph, start: NodeIndex, depth: usize, direction: &Direction, edge_filter: &HashSet<EdgeKind>) -> String` in `src/render.rs`.

The d2 layers feature uses [d2 boards](https://d2lang.com/tour/boards/). The output structure:

```d2
# root board
node_1: "symbolA\n..." {
  link: layers.symbolA
}
node_2: "symbolB\n..."
node_1 -> node_2: "[call]"

layers: {
  symbolA: {
    # depth-1 trace from symbolA
    node_1: "symbolA\n..."
    node_3: "symbolC\n..."
    node_1 -> node_3: "[call]"
  }
}
```

Logic:
1. Render the root board using the existing `render_d2` approach (nodes + edges from the TraceResult).
2. Identify non-leaf nodes: nodes that have at least one edge in the trace. For each non-leaf symbol node, run `traverse::trace(graph, node, 1, direction, edge_filter)` to get its depth-1 neighbourhood.
3. **Redundant layer suppression (AC-3):** Compare the depth-1 trace for a node against the edges already visible on the root board. If the depth-1 trace adds no new nodes or edges beyond what the root board already shows, skip generating a layer for that node.
4. For nodes that pass the redundancy check, add a `link: layers.<sanitised_name>` property to the node declaration on the root board.
5. Append a `layers: { ... }` block at the end with one named board per linked node. Each nested board contains its own node declarations and edges (rendered the same way as `render_d2`).
6. File nodes in `render` command get a layer showing their internal symbols and call edges (see Task 3).

Sanitise symbol names for d2 layer identifiers (replace spaces/special chars with underscores, handle duplicates by appending the node index).

**How to verify:**
```sh
cargo test
# Manual: compile output with d2 and verify click-through works
./target/release/codegraph trace dispatch ~/workspace/tac/apps/web --depth 2 --direction both --layers --output /tmp/test.d2
d2 /tmp/test.d2 /tmp/test.svg
# Open SVG, verify nodes are clickable and navigate to nested boards
```

---

### Task 3: Wire `--layers` through main.rs and support `render --layers`

**ACs addressed:** AC-4 (render --layers drills file nodes into symbol call graphs)

**Files:**
- Modify: `src/main.rs`
- Modify: `src/render.rs`

**What to implement:**

In `src/main.rs`, update the `Trace` command handler (around line 215-217): when `layers` is true and format is D2, call `render::render_d2_layers(...)` instead of `render::render(...)`, passing in `start`, `depth`, `direction`, and `edge_filter`.

For `Render` command handler (around line 236-238): when `layers` is true and format is D2, call a new function `render_d2_file_layers(trace: &TraceResult, graph: &CodeGraph, edge_filter: &HashSet<EdgeKind>) -> String` in `src/render.rs`. This function:
1. Renders the file-level diagram as the root board (same as current `render_d2`).
2. For each file node in the trace, finds all symbol nodes that belong to that file and collects call edges between them.
3. If the file has internal call edges, generates a nested layer board showing the symbol-level call graph within that file.
4. Adds `link: layers.<sanitised_filename>` to file nodes that have layers.

Update `render()` dispatch function signature or add a parallel `render_layered()` entry point.

**How to verify:**
```sh
cargo test
./target/release/codegraph render ~/workspace/tac/packages/logic --layers --output /tmp/render.d2
d2 /tmp/render.d2 /tmp/render.svg
# Open SVG, verify file nodes drill into symbol graphs
```

---

### Task 4: Integration and CLI tests

**ACs addressed:** AC-1, AC-2, AC-3, AC-4, AC-5

**Files:**
- Create: `tests/cli_layers_test.rs`
- Modify: `tests/render_test.rs`

**What to implement:**

**Unit tests in `tests/render_test.rs`:**
- Test `render_d2_layers` produces `layers:` block with nested boards for non-leaf symbols
- Test redundant layer suppression: a leaf node (no further connections) gets no layer
- Test that a node whose depth-1 trace is a subset of the root board gets no layer
- Test `render_d2_file_layers` produces file-to-symbol drill-down layers

**CLI integration tests in `tests/cli_layers_test.rs`:**
- `trace --layers` output contains `layers:` and `link:` directives
- `trace --layers --format mermaid` exits with error
- `trace` without `--layers` produces unchanged output (snapshot comparison)
- `render --layers` output contains `layers:` for files with internal call edges
- `render --layers` on a directory with no internal call edges produces no `layers:` block

Use the same fixture patterns as existing tests (`tempfile::TempDir`, `assert_cmd::Command`).

**How to verify:**
```sh
cargo test
```

## Test Plan

| AC | Test | Type | Properties |
|----|------|------|------------|
| AC-1: nested boards per non-leaf | Assert `layers:` block exists with named sub-boards; each non-leaf symbol has `link:` | Unit + CLI | Behavioral, Specific |
| AC-2: click-through works in compiled d2 | Manual: compile with `d2`, open SVG, verify navigation | Manual | Predictive |
| AC-3: redundant layer suppression | Build graph where a node's depth-1 trace is a subset of root; assert no layer generated | Unit | Behavioral, Deterministic |
| AC-4: render file drill-down | Assert file nodes get `link:` to layers containing symbol-level call edges | Unit + CLI | Behavioral, Specific |
| AC-5: backwards compat | Run trace/render without `--layers`; assert output identical to pre-change | CLI | Structure-insensitive, Predictive |

> Tradeoff: AC-2 (click-through) requires manual verification because programmatically testing d2's SVG board navigation would need a browser automation setup, which isn't justified for this scope. The unit tests cover the d2 syntax correctness; the manual step confirms d2 interprets it correctly.

## Notes

d2 boards syntax reference: a `layers` block at the root level defines named sub-boards. Nodes link to them via `link: layers.<name>`. d2 automatically renders navigation UI between boards.

The `traverse::trace` function is already stateless and reusable -- calling it per-node for layer generation should be straightforward. The main concern is performance on large graphs (many nodes each triggering a depth-1 BFS), but this is bounded by the number of non-leaf nodes in the root trace, which is already constrained by `--depth`.
