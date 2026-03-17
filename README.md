# codegraph

A CLI tool that indexes TypeScript and JavaScript codebases, extracting symbols and call relationships into a graph.

## Usage

```sh
cargo build

# Index a directory — prints symbol and call counts
codegraph index ./path/to/project

# List all discovered symbols
codegraph list ./path/to/project

# Filter by kind: Function, Class, Method, Variable, TypeAlias
codegraph list ./path/to/project --kind Function
```

## Supported languages

JavaScript and TypeScript (`.js`, `.ts`, `.jsx`, `.tsx`).
