---
title: Project indexing
type: iteration
status: accepted
author: agent
date: 2026-03-17
tags: []
related:
- implements: docs/stories/STORY-001-project-indexing.md
---



## Test Plan

- **AC-1 (file discovery):** Index a temp directory with nested `.ts`, `.js`, `.tsx`, `.jsx` files plus a `node_modules/` dir. Assert all non-ignored files are parsed and `node_modules` contents are skipped.
- **AC-2 (symbol extraction):** Index a fixture file containing a function, class, method, variable, and type alias. Assert each produces a `SymbolNode` with correct `name`, `kind`, `file_path`, `line`, and `module`.
- **AC-3 (call edges):** Index a fixture where function `a` calls function `b`. Assert a `Calls` edge exists from `a` to `b` in the graph.
- **AC-4 (list all):** Index a fixture, run `codegraph list`. Assert stdout contains every symbol with kind and file location.
- **AC-5 (list filter):** Index a fixture with mixed kinds, run `codegraph list --kind Function`. Assert only `Function` symbols appear.
- **AC-6 (empty dir):** Index an empty temp directory. Assert zero symbols and an informational message on stderr.
- **AC-7 (partial parse):** Index a fixture with syntax errors. Assert partial symbols are extracted and a warning is emitted without aborting.

## Changes

### Task 1: Project scaffold and graph types

**ACs addressed:** none directly (prerequisite for all)

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/graph.rs`

**What to implement:**
Set up `Cargo.toml` with dependencies: `clap` (derive), `tree-sitter`, `tree-sitter-typescript`, `tree-sitter-javascript`, `petgraph`, `ignore`, `anyhow`. Define the module structure in `main.rs` (`mod walk; mod parse; mod extract; mod graph;`). In `graph.rs`, define `SymbolKind` enum (Function, Class, Method, Variable, TypeAlias), `SymbolNode` struct (name, kind, file_path, line, module), `EdgeKind` enum (Calls), and a `CodeGraph` wrapper around `petgraph::DiGraph<SymbolNode, EdgeKind>` with methods `add_symbol` (returns NodeIndex) and `add_call(from, to)`.

**How to verify:**
`cargo check` passes. Unit test in `src/graph.rs` adds two symbols and a call edge, asserts node/edge counts.

---

### Task 2: Directory walking

**ACs addressed:** AC-1, AC-6

**Files:**
- Create: `src/walk.rs`
- Test: `tests/walk_test.rs`

**What to implement:**
Use the `ignore` crate to walk a directory, respecting `.gitignore` and skipping `node_modules`. Filter entries to files with extensions `.ts`, `.js`, `.tsx`, `.jsx`. Return `Vec<PathBuf>`. If no files found, return empty vec (caller prints informational message).

**How to verify:**
Integration test creates a temp dir with nested TS/JS files and a `node_modules/` dir, calls `walk`, asserts correct files returned and `node_modules` excluded. Second test with empty dir asserts empty vec.

---

### Task 3: Tree-sitter parsing

**ACs addressed:** AC-1, AC-7

**Files:**
- Create: `src/parse.rs`
- Test: `tests/parse_test.rs`

**What to implement:**
Accept a file path, read its contents, select the tree-sitter language (TypeScript for `.ts`/`.tsx`, JavaScript for `.js`/`.jsx`), parse into a `tree_sitter::Tree`. Return `Result<Tree>`. If the tree has errors (`tree.root_node().has_error()`), log a warning to stderr but still return the tree for partial extraction.

**How to verify:**
Test parses a valid TS snippet, asserts tree is returned. Test parses a broken snippet, asserts tree is returned and `has_error()` is true.

---

### Task 4: Symbol and call extraction

**ACs addressed:** AC-2, AC-3, AC-7

**Files:**
- Create: `src/extract.rs`
- Test: `tests/extract_test.rs`

**What to implement:**
Walk the CST using a cursor. Match node kinds: `function_declaration`, `class_declaration`, `method_definition`, `lexical_declaration`/`variable_declaration`, `type_alias_declaration` to extract `SymbolNode` values. For call extraction, match `call_expression` and `member_expression` call patterns, resolve caller (nearest enclosing function/method) and callee (identifier text). Return a struct `ExtractionResult { symbols: Vec<SymbolNode>, calls: Vec<(String, String)> }`. Partial extraction: skip nodes that fail to resolve names, continue walking.

**How to verify:**
Test extracts symbols from a fixture with all five kinds, asserts correct names/kinds/lines. Test extracts a call edge from `function a() { b(); }`, asserts `("a", "b")` in calls. Test with malformed file asserts partial results returned.

---

### Task 5: CLI commands (index + list)

**ACs addressed:** AC-1, AC-4, AC-5, AC-6

**Files:**
- Modify: `src/main.rs`
- Test: `tests/cli_test.rs`

**What to implement:**
Use `clap` derive to define two subcommands. `index <path>`: call `walk`, `parse` each file, `extract` symbols/calls, build `CodeGraph`, print summary (`Indexed N symbols, M calls`). If zero files found, print informational message. `list [--kind <kind>]`: after indexing, print each symbol as `kind  name  file:line`. If `--kind` is provided, filter by that kind. For this iteration, `list` implies a prior `index` in the same invocation (pass path to both).

**How to verify:**
CLI integration test runs `codegraph index` on a fixture dir via `assert_cmd`, checks exit code 0 and summary output. Runs `codegraph list` and `codegraph list --kind Function`, asserts correct filtered output. Runs on empty dir, asserts informational message.

## Notes

Module dependency order: `graph` (no deps) -> `walk` (ignore) -> `parse` (tree-sitter) -> `extract` (parse + graph) -> `main` (all). Tasks can be executed in order 1-5. Tasks 2 and 3 are independent of each other and could be parallelized.
