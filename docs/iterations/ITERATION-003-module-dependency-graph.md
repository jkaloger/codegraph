---
title: Module dependency graph
type: iteration
status: draft
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/stories/STORY-003-module-dependency-graph.md
---


## Test Plan

- **AC-1 (ES import):** Fixture with `import { foo } from './bar'`. Assert an `Imports` edge from the importing file node to the resolved target file node.
- **AC-2 (require):** Fixture with `const x = require('./bar')`. Assert an `Imports` edge identical to ES import.
- **AC-3 (dynamic import):** Fixture with `const m = import('./bar')`. Assert an `Imports` edge with same semantics.
- **AC-4 (exports):** Fixture with named export, default export, and `export { x } from './mod'`. Assert `Exports` edges for re-exports; local exports recorded as file node metadata.
- **AC-5 (relative resolution):** Fixture dir with `./foo` resolving to `foo.ts`, `./bar` resolving to `bar/index.tsx`. Assert resolver returns correct absolute paths, trying `.ts/.tsx/.js/.jsx` and `/index.*`.
- **AC-6 (bare specifier):** Import of `lodash`. Assert resolver returns `Unresolved("lodash")` rather than a file path.
- **AC-7 (--kind import):** Build a graph with both `Calls` and `Imports` edges. Run `codegraph trace --kind import`. Assert output includes only import/export edges.
- **AC-8 (file diagram):** Index a multi-file fixture. Run `codegraph render`. Assert output contains file-level nodes and import edges.

## Changes

### Task 1: Extend graph with import/export edge types

**ACs addressed:** AC-1, AC-2, AC-3, AC-4
**Files:**
- Modify: `src/graph.rs`
- Test: `tests/graph_test.rs`

**What to implement:**
Add `Imports` and `Exports` variants to the `EdgeKind` enum. Add a `FileNode` struct (file_path, exports metadata vec). Add `add_file(path) -> NodeIndex` and `add_import(from, to)` / `add_export(from, to)` methods to `CodeGraph`. File nodes are deduplicated by path. The graph now holds both `SymbolNode` and `FileNode` as a `NodeKind` enum wrapping both.

**How to verify:**
Unit test adds two file nodes and an import edge, asserts edge kind is `Imports` and node count is 2.

---

### Task 2: Module path resolver

**ACs addressed:** AC-5, AC-6
**Files:**
- Create: `src/resolve.rs`
- Test: `tests/resolve_test.rs`

**What to implement:**
`resolve_specifier(specifier: &str, importer_path: &Path) -> ResolveResult` where `ResolveResult` is `Resolved(PathBuf)` or `External(String)`. For relative specifiers (starting with `./` or `../`), join with importer's parent dir, then probe for existence in order: exact path, `.ts`, `.tsx`, `.js`, `.jsx`, then `/index.ts`, `/index.tsx`, `/index.js`, `/index.jsx`. Return first match. For bare specifiers, return `External(specifier)`.

**How to verify:**
Integration test creates a temp dir with `foo.ts` and `bar/index.tsx`. Asserts `resolve("./foo", ...)` yields `foo.ts`, `resolve("./bar", ...)` yields `bar/index.tsx`, and `resolve("lodash", ...)` yields `External("lodash")`.

---

### Task 3: Import/export extraction from CST

**ACs addressed:** AC-1, AC-2, AC-3, AC-4
**Files:**
- Modify: `src/extract.rs`
- Test: `tests/extract_test.rs`

**What to implement:**
Extend the CST visitor to match additional node kinds: `import_statement` (ES import), `call_expression` where callee is `require` (CJS), `call_expression` where callee is `import` (dynamic import), `export_statement`, and `export_default_declaration`. For each import pattern, extract the string literal specifier. For exports, detect re-exports (`export { x } from '...'`) vs local exports. Return `ImportResult { imports: Vec<(String, String)>, exports: Vec<ExportEntry> }` alongside the existing `ExtractionResult`. `ExportEntry` holds kind (named/default/reexport) and optional source specifier.

**How to verify:**
Test parses a fixture containing all three import forms and all export forms. Assert correct specifiers extracted for each. Test with a file containing no imports asserts empty results.

---

### Task 4: Wire extraction through resolver into graph

**ACs addressed:** AC-1, AC-2, AC-3, AC-4, AC-5, AC-6
**Files:**
- Modify: `src/main.rs`
- Test: `tests/integration_test.rs`

**What to implement:**
After extracting imports/exports per file, pass each specifier through `resolve_specifier`. For `Resolved` results, call `graph.add_file` for both source and target, then `graph.add_import(src, tgt)`. For re-exports, also add the target file node and an `Exports` edge. For `External` results, optionally record in a side list (no graph edge). Update the `index` subcommand summary to include import/export edge counts.

**How to verify:**
Integration test indexes a fixture dir with two files where `a.ts` imports `./b`. Assert graph contains two file nodes and one `Imports` edge. Assert bare specifier `lodash` does not create a graph edge.

---

### Task 5: --kind import filtering and file-level diagram

**ACs addressed:** AC-7, AC-8
**Files:**
- Modify: `src/main.rs`
- Test: `tests/cli_test.rs`

**What to implement:**
Extend the `--kind` flag on `trace` to accept `import` as a value. When `--kind import` is active, filter the graph to only `Imports` and `Exports` edges before traversal. For the `render` subcommand, add a file-level mode (default when `--kind import`): output DOT format where each node is a file path and each edge is an import/re-export relationship. Reuse the existing render pipeline from ITER-002.

**How to verify:**
CLI test builds a fixture with both call and import edges. `codegraph trace --kind import` output includes import edges only. `codegraph render --kind import` output is valid DOT with file-path nodes.

## Notes

Depends on ITER-001 (graph types, extraction framework, walk/parse) and ITER-002 (trace/render pipeline). Tasks 1 and 2 are independent and can be parallelized. Tasks 3 depends on 1. Task 4 depends on 2 and 3. Task 5 depends on 4.
