use std::path::Path;

use tree_sitter::Tree;

use crate::graph::{SymbolKind, SymbolNode};

#[derive(Clone, Debug, PartialEq)]
pub enum RefKind {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceEntry {
    pub symbol_name: String,
    pub kind: RefKind,
    pub file_path: String,
    pub line: usize,
    pub enclosing_scope: Option<String>,
}

pub struct ExtractionResult {
    pub symbols: Vec<SymbolNode>,
    pub calls: Vec<(String, String)>,
    pub imports: Vec<String>,
    pub reexports: Vec<String>,
    pub references: Vec<ReferenceEntry>,
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
    let mut references = Vec::new();

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
        &mut references,
    );

    ExtractionResult { symbols, calls, imports, reexports, references }
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
    references: &mut Vec<ReferenceEntry>,
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
                extract_declaration_references(&node, source, file_path, references);
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
            "assignment_expression" | "augmented_assignment_expression" => {
                extract_assignment_references(&node, source, file_path, references);
            }
            "update_expression" => {
                extract_update_references(&node, source, file_path, references);
            }
            "identifier" | "shorthand_property_identifier" => {
                if is_read_reference(&node, source) {
                    let name = source[node.byte_range()].to_string();
                    references.push(ReferenceEntry {
                        symbol_name: name,
                        kind: RefKind::Read,
                        file_path: file_path.to_string(),
                        line: node.start_position().row + 1,
                        enclosing_scope: enclosing_scope_name(&node, source),
                    });
                }
            }
            _ => {}
        }

        if cursor.goto_first_child() {
            walk_node(cursor, source, file_path, module, symbols, calls, imports, reexports, references);
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

fn extract_declaration_references(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    references: &mut Vec<ReferenceEntry>,
) {
    let scope = enclosing_scope_name(node, source);
    let mut child_cursor = node.walk();
    for child in node.children(&mut child_cursor) {
        if child.kind() != "variable_declarator" {
            continue;
        }
        let Some(name_node) = child.child_by_field_name("name") else {
            continue;
        };

        match name_node.kind() {
            "identifier" => {
                references.push(ReferenceEntry {
                    symbol_name: source[name_node.byte_range()].to_string(),
                    kind: RefKind::Write,
                    file_path: file_path.to_string(),
                    line: name_node.start_position().row + 1,
                    enclosing_scope: scope.clone(),
                });
            }
            "object_pattern" | "array_pattern" => {
                extract_pattern_write_references(&name_node, source, file_path, &scope, references);
            }
            _ => {}
        }
    }
}

fn extract_pattern_write_references(
    pattern: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    scope: &Option<String>,
    references: &mut Vec<ReferenceEntry>,
) {
    let mut cursor = pattern.walk();
    for child in pattern.children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                references.push(ReferenceEntry {
                    symbol_name: source[child.byte_range()].to_string(),
                    kind: RefKind::Write,
                    file_path: file_path.to_string(),
                    line: child.start_position().row + 1,
                    enclosing_scope: scope.clone(),
                });
            }
            "pair_pattern" => {
                if let Some(value) = child.child_by_field_name("value") {
                    if value.kind() == "identifier" {
                        references.push(ReferenceEntry {
                            symbol_name: source[value.byte_range()].to_string(),
                            kind: RefKind::Write,
                            file_path: file_path.to_string(),
                            line: value.start_position().row + 1,
                            enclosing_scope: scope.clone(),
                        });
                    }
                }
            }
            "object_pattern" | "array_pattern" => {
                extract_pattern_write_references(&child, source, file_path, scope, references);
            }
            _ => {}
        }
    }
}

fn extract_assignment_references(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    references: &mut Vec<ReferenceEntry>,
) {
    let Some(left) = node.child_by_field_name("left") else {
        return;
    };
    if left.kind() == "identifier" {
        references.push(ReferenceEntry {
            symbol_name: source[left.byte_range()].to_string(),
            kind: RefKind::Write,
            file_path: file_path.to_string(),
            line: left.start_position().row + 1,
            enclosing_scope: enclosing_scope_name(node, source),
        });
    }
}

fn extract_update_references(
    node: &tree_sitter::Node,
    source: &str,
    file_path: &str,
    references: &mut Vec<ReferenceEntry>,
) {
    let Some(argument) = node.child_by_field_name("argument") else {
        return;
    };
    if argument.kind() == "identifier" {
        references.push(ReferenceEntry {
            symbol_name: source[argument.byte_range()].to_string(),
            kind: RefKind::Write,
            file_path: file_path.to_string(),
            line: argument.start_position().row + 1,
            enclosing_scope: enclosing_scope_name(node, source),
        });
    }
}

fn is_read_reference(node: &tree_sitter::Node, source: &str) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };

    match parent.kind() {
        // Symbol definition names are not references
        "function_declaration" | "class_declaration" | "method_definition"
        | "type_alias_declaration" => {
            return is_not_name_field(node, &parent);
        }
        // Variable declarator name is handled as a write in extract_declaration_references
        "variable_declarator" => {
            let is_name = parent
                .child_by_field_name("name")
                .map_or(false, |n| n.id() == node.id());
            return !is_name;
        }
        // LHS of assignment handled by extract_assignment_references
        "assignment_expression" | "augmented_assignment_expression" => {
            let is_left = parent
                .child_by_field_name("left")
                .map_or(false, |n| n.id() == node.id());
            return !is_left;
        }
        // Update expression operand handled by extract_update_references
        "update_expression" => return false,
        // Destructuring patterns handled by extract_declaration_references
        "object_pattern" | "array_pattern" | "shorthand_property_identifier_pattern" => {
            return false;
        }
        "pair_pattern" => return false,
        // Call target identifier: `foo` in `foo()` is the callee, not a read ref
        "call_expression" => {
            let is_callee = parent
                .child_by_field_name("function")
                .map_or(false, |n| n.id() == node.id());
            return !is_callee;
        }
        // Import specifiers are not references
        "import_specifier" | "import_clause" | "namespace_import" => return false,
        // Property access keys like `obj.prop` - prop is not a standalone reference
        "member_expression" => {
            let is_property = parent
                .child_by_field_name("property")
                .map_or(false, |n| n.id() == node.id());
            return !is_property;
        }
        // `import` keyword itself
        "import" => return false,
        // Formal parameters are not read references
        "formal_parameters" | "required_parameter" | "optional_parameter" => return false,
        _ => {}
    }

    // Ignore keywords that tree-sitter may parse as identifiers
    let text = &source[node.byte_range()];
    if matches!(text, "undefined" | "null" | "true" | "false" | "this" | "super") {
        return false;
    }

    true
}

fn enclosing_scope_name(node: &tree_sitter::Node, source: &str) -> Option<String> {
    find_enclosing_function(node).map(|name_node| source[name_node.byte_range()].to_string())
}

fn is_not_name_field(node: &tree_sitter::Node, parent: &tree_sitter::Node) -> bool {
    let is_name = parent
        .child_by_field_name("name")
        .map_or(false, |n| n.id() == node.id());
    !is_name
}
