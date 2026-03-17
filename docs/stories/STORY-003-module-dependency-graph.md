---
title: Module dependency graph
type: story
status: draft
author: unknown
date: 2026-03-17
tags: []
related:
- implements: docs/rfcs/RFC-001-idea.md
---


## Context

`codegraph` needs to understand module-level dependencies between files so users can visualise how their TypeScript/JS project is wired together at the file level. This story covers extracting import/export statements from tree-sitter CSTs, resolving module paths, storing the resulting edges in petgraph, and rendering a file-level dependency diagram. It builds on the parsing and graph infrastructure from Story 1 and reuses the traversal/rendering pipeline from Story 2.

## Acceptance Criteria

- **Given** a TypeScript file containing ES module `import` declarations
  **When** the extractor visits the CST
  **Then** an `imports` edge is added from the importing file node to the resolved target file node in the graph

- **Given** a JavaScript file containing `require()` calls
  **When** the extractor visits the CST
  **Then** an `imports` edge is added from the requiring file to the resolved target file, identical to an ES import edge

- **Given** a file containing a dynamic `import()` expression
  **When** the extractor visits the CST
  **Then** an `imports` edge is added with the same semantics as a static import declaration

- **Given** a file containing named exports, default exports, or re-exports (`export { x } from './mod'`)
  **When** the extractor visits the CST
  **Then** an `exports` edge is added from the exporting file to the target file (for re-exports) or the export is recorded as metadata on the file node

- **Given** an import with a relative path specifier (e.g. `./foo`, `../bar`)
  **When** the module path resolver runs
  **Then** the specifier is resolved to an absolute file path relative to the importing file, trying `.ts`, `.tsx`, `.js`, `.jsx`, and `/index.*` extensions

- **Given** an import with a bare specifier (e.g. `lodash`, `@org/pkg`)
  **When** the module path resolver runs
  **Then** the specifier is recorded as an unresolved external dependency (node_modules resolution is out of scope)

- **Given** a project with multiple files and import/export edges in the graph
  **When** the user runs `codegraph trace --kind import`
  **Then** only import and export edges are included in the output, and call/reference edges are excluded

- **Given** a project with import/export edges in the graph
  **When** the user runs `codegraph render`
  **Then** a file-level dependency diagram is produced where each node is a file and each edge represents an import or re-export relationship

## Scope

### In Scope

- Extraction of import statements: ES module `import` declarations, dynamic `import()` expressions, CommonJS `require()` calls
- Extraction of export statements: named exports, default exports, re-exports
- Module path resolution for relative paths (with extension and index file probing)
- Recording bare specifiers as unresolved external dependencies
- Adding `imports` and `exports` edge types to the petgraph
- File-level (coarse-grained) dependency diagram rendering
- Filtering trace output via `--kind import`

### Out of Scope

- Symbol-level extraction and call edges (Story 1)
- Traversal and rendering infrastructure (Story 2, reused here)
- Reference edges (Story 4)
- Full `node_modules` resolution (future work)
- Package.json `exports`/`imports` field resolution
- Path alias resolution (tsconfig `paths`)
