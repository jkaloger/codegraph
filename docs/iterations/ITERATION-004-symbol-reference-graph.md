---
title: Symbol reference graph
type: iteration
status: accepted
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/stories/STORY-004-symbol-reference-graph.md
---



## Test Plan

- **AC-1 (reference detection):** Parse a fixture containing `let x = 1; console.log(x); x = 2;`. Assert the extractor produces reference entries for `x` classified as write (declaration), read (log arg), and write (reassignment).
- **AC-2 (read vs write classification):** Parse `x = foo` and `bar(x)`. Assert `x` has a write-reference edge from the assignment and a read-reference edge from the call argument. Assert `foo` has a read-reference edge from the assignment RHS.
- **AC-3 (trace --kind ref):** Build a graph with both call and reference edges. Run `codegraph trace x --kind ref`. Assert only reference edges appear; no call edges in output.
- **AC-4 (trace --kind all):** Same graph as AC-3. Run `codegraph trace x --kind all`. Assert both call edges and reference edges appear.
- **AC-5 (call-only filter excludes refs):** Define a symbol `y` that is read/written but never called. Run `codegraph trace y --kind call`. Assert empty result.
- **AC-6 (destructuring):** Parse `const { a, b } = obj;`. Assert `a` and `b` each have write-reference edges from the destructuring site. Assert `obj` has a read-reference edge.

## Changes

### Task 1: Extend graph edge types with reference variants

**ACs addressed:** AC-1, AC-2
**Files:**
- Modify: `src/graph.rs`
- Test: `tests/graph_test.rs`

**What to implement:**
Add `ReadsFrom` and `WritesTo` variants to the `EdgeKind` enum. Add a `add_reference(from: NodeIndex, to: NodeIndex, kind: EdgeKind)` method to `CodeGraph` that validates the edge kind is a reference type before inserting. Add a `references_of(node: NodeIndex) -> Vec<(NodeIndex, &EdgeKind)>` query method that filters edges to only reference types.

**How to verify:**
Unit test creates two symbol nodes, adds `ReadsFrom` and `WritesTo` edges, asserts edge counts. Test that `references_of` returns only reference edges when call edges also exist.

---

### Task 2: Reference site extraction from CST

**ACs addressed:** AC-1, AC-2, AC-6
**Files:**
- Modify: `src/extract.rs`
- Test: `tests/extract_test.rs`

**What to implement:**
Extend the CST visitor to detect reference sites. Walk `identifier` and `shorthand_property_identifier` nodes that are not already captured as symbol definitions or call targets. Classify each as read or write by inspecting the parent node:

- **Write contexts:** LHS of `assignment_expression`, `variable_declarator` name, `augmented_assignment_expression` LHS, `update_expression` operand, destructuring patterns (`object_pattern`, `array_pattern` children).
- **Read contexts:** everything else (RHS of assignments, function arguments, return values, template literals, property access base).

Produce `ReferenceEntry { symbol_name: String, kind: RefKind, file_path, line }` where `RefKind` is `Read` or `Write`. Add these to `ExtractionResult` as a new `references: Vec<ReferenceEntry>` field.

For destructuring (`const { a, b } = obj`), iterate pattern children and emit a write-reference for each bound name. Emit a read-reference for `obj`.

**How to verify:**
Test fixture `let x = 1; console.log(x); x = 2;` produces 1 write (decl), 1 read (log arg), 1 write (reassign) for `x`. Test fixture `const { a, b } = obj;` produces write refs for `a` and `b`, read ref for `obj`. Test that call-target identifiers (e.g. `foo` in `foo()`) are not double-counted as read references.

---

### Task 3: Wire reference edges into graph building

**ACs addressed:** AC-1, AC-2
**Files:**
- Modify: `src/extract.rs` (graph-building pass)
- Modify: `src/main.rs` (summary output)
- Test: `tests/extract_test.rs`

**What to implement:**
After extraction, resolve each `ReferenceEntry` to a `NodeIndex` by looking up the symbol name in the graph. If the referenced symbol exists, add a `ReadsFrom` or `WritesTo` edge between the referencing context (enclosing function/module scope) and the referenced symbol. Update the index summary to print `Indexed N symbols, M calls, R references`.

**How to verify:**
Integration test indexes a fixture with `function a() { let x = 1; b(x); x = 2; }`. Assert graph contains `ReadsFrom` edge for the `b(x)` usage and `WritesTo` edges for declaration and reassignment. Assert summary line includes reference count.

---

### Task 4: Extend trace CLI with --kind ref and --kind all

**ACs addressed:** AC-3, AC-4, AC-5
**Files:**
- Modify: `src/main.rs` (CLI argument parsing)
- Modify: `src/graph.rs` (trace filtering)
- Test: `tests/cli_test.rs`

**What to implement:**
Extend the `--kind` argument on `codegraph trace` to accept `ref` and `all` in addition to the existing `call`. When `--kind ref`, the trace traversal follows only `ReadsFrom`/`WritesTo` edges. When `--kind all`, follow both call and reference edges. When `--kind call` (existing default), follow only `Calls` edges, meaning reference-only symbols return no results. In trace output, prefix reference edges with `[read]` or `[write]` to distinguish from `[call]` edges.

**How to verify:**
CLI test builds a fixture with both call and reference edges. `codegraph trace x --kind ref` returns only `[read]`/`[write]` lines. `codegraph trace x --kind all` returns both `[call]` and `[read]`/`[write]` lines. `codegraph trace y --kind call` (where `y` is ref-only) returns empty output with exit code 0.

## Notes

Task dependency: 1 -> 2 -> 3 -> 4 (linear). Task 1 is small and could be done alongside task 2 if preferred, since the edge types are straightforward.

Key design decision: reference edges point from the *usage context* (enclosing function or module scope) to the *referenced symbol*. This matches the call edge direction (caller -> callee) and keeps traversal consistent.

Limitation: identifier resolution is name-based, not scope-aware. Shadowed variables in nested scopes may produce incorrect edges. This is acceptable for the MVP; scope-aware resolution is a future concern.
