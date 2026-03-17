use std::path::Path;

use anyhow::{bail, Context, Result};
use tree_sitter::{Parser, Tree};

pub fn parse_file(path: &Path) -> Result<Tree> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let language = match ext {
        "ts" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "js" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        _ => bail!("unsupported file extension: .{ext}"),
    };

    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let mut parser = Parser::new();
    parser.set_language(&language)?;

    let tree = parser
        .parse(&source, None)
        .context("tree-sitter parse returned None")?;

    if tree.root_node().has_error() {
        eprintln!(
            "warning: parse errors in {}",
            path.display()
        );
    }

    Ok(tree)
}
