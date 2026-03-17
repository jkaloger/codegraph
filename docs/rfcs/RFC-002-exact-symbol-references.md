---
title: exact symbol references
type: rfc
status: draft
author: unknown
date: 2026-03-17
tags:
- rust
- tree-sitter
- developer-tools
related:
- related-to: docs/rfcs/RFC-001-idea.md
---


## Problem

codegraph's MVP resolves symbols by name only. When multiple modules define a symbol with the same name (e.g. two functions called `handle` in different files), the graph produces ambiguous edges. Queries against ambiguous symbols return a merged neighbourhood that conflates unrelated call chains.

RFC-001 acknowledges this as a known limitation: "No type-level resolution. If `foo()` is called on a variable, we resolve it by name, not by type."

This RFC introduces module-qualified symbol identifiers to eliminate that ambiguity, both in the stored graph and in CLI queries.

## Intent

Every symbol in the graph should be uniquely addressable by its file path and name. A developer querying `codegraph trace` should never receive a merged result from two unrelated symbols that happen to share a name.

The qualifying syntax is `file_path#symbol`, where `file_path` is the relative path from the project root:

```
codegraph trace src/utils/math.ts#add --depth 2
```

This applies end-to-end: the indexer resolves edges to module-qualified nodes during extraction, and the CLI requires qualified identifiers in queries.

## Design

### Module Identity

A module is identified by its relative file path from the project root. This is the simplest unambiguous identifier available without requiring `tsconfig.json` resolution or package metadata.

Symbol identity becomes the tuple `(file_path, name, kind)`. The `@ref` RFC-001 `SymbolNode` struct already carries `file_path`, `name`, and `kind` fields. The change is that edge resolution during extraction uses the full tuple rather than name-only matching.

### Import Resolution

When the extractor encounters a call expression like `foo()`, it must determine which module's `foo` is being invoked. This requires following the import chain:

1. Find the import statement that brings `foo` into scope
2. Resolve the import specifier (`'./bar'`) to a file path (`src/bar.ts`)
3. If `bar.ts` re-exports from another module, follow the chain
4. Create the edge targeting the resolved module's symbol node

Resolution rules:

- **Named imports**: `import { foo } from './bar'` resolves `foo` to `bar.ts#foo`
- **Re-exports**: `export { foo } from './baz'` chains through to `baz.ts#foo`
- **Barrel files**: `import { foo } from './utils'` where `utils/index.ts` re-exports from `utils/math.ts` resolves to `utils/math.ts#foo`
- **Default imports**: `import bar from './bar'` resolves to `bar.ts#default`
- **Namespace imports**: `import * as utils from './utils'` followed by `utils.foo()` resolves to `utils.ts#foo`
- **External packages**: `import { useState } from 'react'` resolves to `<external:react>#useState`
- **Unresolvable**: Dynamic imports, computed property access, and other patterns that can't be statically resolved get tagged as `<unresolved>#symbol`

> [!NOTE]
> This is syntactic import resolution, not TypeScript module resolution. We follow import statements as written and resolve relative paths. We do not read `tsconfig.json` paths, `baseUrl`, or `moduleResolution` settings. This keeps the zero-config promise from RFC-001.

### Interface Sketches

The existing `@ref` RFC-001 `SymbolNode` struct gains no new fields. The `module` field (relative file path) is already present. What changes is how it's used:

```
@draft ImportResolution {
    source_file: PathBuf,       // file containing the import
    specifier: String,          // raw import path (e.g. './bar')
    resolved_file: PathBuf,     // resolved file path (e.g. src/bar.ts)
    symbols: Vec<ImportedSymbol>, // which symbols are imported
}

@draft ImportedSymbol {
    local_name: String,         // name in the importing file
    original_name: String,      // name in the exporting file (may differ with aliases)
    kind: ImportKind,           // Named, Default, Namespace
}

@draft ImportKind {
    Named,                      // import { foo } from './bar'
    Default,                    // import foo from './bar'
    Namespace,                  // import * as foo from './bar'
}
```

### Query Syntax

All `codegraph trace` and `codegraph list` queries use the `file_path#symbol` format:

```
codegraph trace src/handlers/auth.ts#handle --depth 2
codegraph list --kind function    # outputs qualified names
```

The `#` delimiter is unambiguous because `#` is not a valid character in file paths or JS/TS symbol names. The CLI splits on the first `#` occurrence: everything before is the file path, everything after is the symbol name.

`codegraph list` serves as the discovery mechanism. When a developer doesn't know the qualified name, they list symbols and filter:

```
codegraph list --kind function | grep handle
# src/handlers/auth.ts#handle
# src/handlers/order.ts#handle
# src/middleware/error.ts#handle
```

### Limitations

This RFC addresses module-level ambiguity only. The following remain out of scope:

- **Type-level resolution**: Method calls on typed variables (e.g. `user.save()`) still resolve by name, not by the type of `user`. Solving this requires type inference.
- **Dynamic dispatch**: Callbacks, higher-order functions, and computed property access cannot be statically traced.
- **Runtime aliasing**: `const f = foo; f()` creates a call edge from the alias site, but `f` is not traced back to `foo`.

## Stories

1. **Import resolution engine**: Build the import chain follower that resolves import statements to file paths. Handles named imports, re-exports, barrel files, default imports, namespace imports, and external/unresolvable fallbacks.

2. **Module-qualified graph storage**: Update the extractor to resolve call edges using import resolution, creating edges to module-qualified symbol nodes instead of name-matched nodes.

3. **Qualified query syntax**: Update the CLI to require and parse `file_path#symbol` queries across `trace` and `list` commands.
