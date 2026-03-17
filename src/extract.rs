use std::path::Path;

use tree_sitter::Tree;

use crate::graph::{SymbolKind, SymbolNode};

pub struct ExtractionResult {
    pub symbols: Vec<SymbolNode>,
    pub calls: Vec<(String, String)>,
    pub imports: Vec<String>,
    pub reexports: Vec<String>,
}

pub fn extract(tree: &Tree, source: &str, file_path: &Path) -> ExtractionResult {
    let file_path_str = file_path.to_string_lossy().to_string();
    let module = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let mut symbols = Vec::new();
    let mut calls = Vec::new();
    let mut imports = Vec::new();
    let mut reexports = Vec::new();

    let mut cursor = tree.walk();
    walk_node(
        &mut cursor,
        source,
        &file_path_str,
        &module,
        &mut symbols,
        &mut calls,
        &mut imports,
        &mut reexports,
    );

    ExtractionResult { symbols, calls, imports, reexports }
}

fn walk_node(
    cursor: &mut tree_sitter::TreeCursor,
    source: &str,
    file_path: &str,
    module: &str,
    symbols: &mut Vec<SymbolNode>,
    calls: &mut Vec<(String, String)>,
    imports: &mut Vec<String>,
    reexports: &mut Vec<String>,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        match kind {
            "function_declaration" => {
                if let Some(name) = node_field_text(&node, "name", source) {
                    symbols.push(SymbolNode {
                        name,
                        kind: SymbolKind::Function,
                        file_path: file_path.to_string(),
                        line: node.start_position().row + 1,
                        module: module.to_string(),
                    });
                }
            }
            "class_declaration" => {
                if let Some(name) = node_field_text(&node, "name", source) {
                    symbols.push(SymbolNode {
                        name,
                        kind: SymbolKind::Class,
                        file_path: file_path.to_string(),
                        line: node.start_position().row + 1,
                        module: module.to_string(),
                    });
                }
            }
            "method_definition" => {
                if let Some(name) = node_field_text(&node, "name", source) {
                    symbols.push(SymbolNode {
                        name,
                        kind: SymbolKind::Method,
                        file_path: file_path.to_string(),
                        line: node.start_position().row + 1,
                        module: module.to_string(),
                    });
                }
            }
            "lexical_declaration" | "variable_declaration" => {
                extract_variable_declarators(&node, source, file_path, module, symbols);
            }
            "type_alias_declaration" => {
                if let Some(name) = node_field_text(&node, "name", source) {
                    symbols.push(SymbolNode {
                        name,
                        kind: SymbolKind::TypeAlias,
                        file_path: file_path.to_string(),
                        line: node.start_position().row + 1,
                        module: module.to_string(),
                    });
                }
            }
            "import_statement" => {
                if let Some(specifier) = extract_string_field(&node, "source", source) {
                    imports.push(specifier);
                }
            }
            "export_statement" => {
                if let Some(specifier) = extract_string_field(&node, "source", source) {
                    reexports.push(specifier);
                }
            }
            "call_expression" => {
                if is_import_call(&node, source) {
                    if let Some(specifier) = extract_first_string_arg(&node, source) {
                        imports.push(specifier);
                    }
                } else if let Some(callee_name) = resolve_callee(&node, source) {
                    if let Some(caller_name) = find_enclosing_function(&node) {
                        let caller_text = &source[caller_name.byte_range()];
                        calls.push((caller_text.to_string(), callee_name));
                    }
                }
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            walk_node(cursor, source, file_path, module, symbols, calls, imports, reexports);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn node_field_text(node: &tree_sitter::Node, field: &str, source: &str) -> Option<String> {
    let child = node.child_by_field_name(field)?;
    Some(source[child.byte_range()].to_string())
}

fn extract_variable_declarators(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    module: &str,
    symbols: &mut Vec<SymbolNode>,
) {
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name) = node_field_text(&child, "name", source) {
                symbols.push(SymbolNode {
                    name,
                    kind: SymbolKind::Variable,
                    file_path: file_path.to_string(),
                    line: child.start_position().row + 1,
                    module: module.to_string(),
                });
            }
        }
    }
}

fn resolve_callee(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let func_node = node.child_by_field_name("function")?;
    match func_node.kind() {
        "identifier" => Some(source[func_node.byte_range()].to_string()),
        "member_expression" => {
            // For `this.helper()` or `obj.method()`, extract the property name
            let property = func_node.child_by_field_name("property")?;
            Some(source[property.byte_range()].to_string())
        }
        _ => None,
    }
}

fn is_import_call(node: &tree_sitter::Node, source: &str) -> bool {
    if let Some(func) = node.child_by_field_name("function") {
        return func.kind() == "import"
            || (func.kind() == "identifier" && &source[func.byte_range()] == "require");
    }
    false
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches('"').trim_matches('\'').to_string()
}

fn extract_string_field(node: &tree_sitter::Node, field: &str, source: &str) -> Option<String> {
    let child = node.child_by_field_name(field)?;
    if child.kind() == "string" || child.kind().contains("string") {
        Some(strip_quotes(&source[child.byte_range()]))
    } else {
        None
    }
}

fn extract_first_string_arg(node: &tree_sitter::Node, source: &str) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    for child in args.children(&mut cursor) {
        if child.kind() == "string" || child.kind().contains("string") {
            return Some(strip_quotes(&source[child.byte_range()]));
        }
    }
    None
}

fn find_enclosing_function<'a>(node: &tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
    let mut current = node.parent();
    while let Some(n) = current {
        match n.kind() {
            "function_declaration" | "method_definition" => {
                return n.child_by_field_name("name");
            }
            _ => current = n.parent(),
        }
    }
    None
}
