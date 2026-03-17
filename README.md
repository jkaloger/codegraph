# codegraph

A CLI tool that indexes TypeScript and JavaScript codebases, extracting symbols and call relationships into a navigable graph. Built with tree-sitter and petgraph.

## Install

```sh
cargo build --release
```

## Commands

### `index` -- summarise a codebase

Prints symbol, call, import, and export counts for a directory.

```sh
codegraph index ./src
# Indexed 45 symbols, 123 calls, 8 imports, 2 exports
```

### `list` -- enumerate symbols

Tab-separated list of every symbol with its kind, name, and location.

```sh
codegraph list ./src

# Filter by kind: Function, Class, Method, Variable, TypeAlias
codegraph list ./src --kind Function
```

### `trace` -- visualise a symbol's relationships

Produces a graph diagram rooted at a named symbol, showing callers, callees, or both.

```sh
# Default: depth 2, both directions, d2 format
codegraph trace myFunction ./src

# Outbound calls only, 3 levels deep, mermaid output to file
codegraph trace myFunction ./src --depth 3 --direction out --format mermaid --output graph.md
```

| Flag | Values | Default |
|------|--------|---------|
| `--depth` | integer | 2 |
| `--direction` | `in`, `out`, `both` | `both` |
| `--kind` | `call`, `import`, `all` | `call` |
| `--format` | `d2`, `mermaid` | `d2` |
| `--output` | file path | stdout |

If the symbol name is ambiguous, codegraph lists all matches with file locations so you can narrow it down.

### `render` -- file-level dependency diagram

Generates a module dependency graph where each node is a file.

```sh
codegraph render ./src
codegraph render ./src --kind all --format mermaid --output deps.md
```

| Flag | Values | Default |
|------|--------|---------|
| `--kind` | `import`, `all` | `import` |
| `--format` | `d2`, `mermaid` | `d2` |
| `--output` | file path | stdout |

## Supported languages

JavaScript and TypeScript (`.js`, `.ts`, `.jsx`, `.tsx`).

`node_modules` is excluded automatically. Bare specifiers (e.g. `lodash`) are recorded as external dependencies but not resolved into `node_modules`.

## Output formats

**D2** -- text-based diagram syntax, renderable with the [d2 CLI](https://d2lang.com).

**Mermaid** -- Markdown-compatible flowcharts, renderable in GitHub, GitLab, and most documentation tools.
