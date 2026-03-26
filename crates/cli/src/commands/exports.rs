use anyhow::Result;
use ast_grep_language::SupportLang;
use std::path::Path;

use crate::ast::engine::{detect_lang, parse_file};

/// An exported symbol found in a file.
struct ExportedSymbol {
    name: String,
    kind: String,  // e.g. "function", "class", "const", "type", "interface"
    line: usize,   // 0-based
}

pub fn run(path: &Path) -> Result<()> {
    if !path.exists() {
        anyhow::bail!("Path does not exist: {}", path.display());
    }

    if path.is_file() {
        print_exports(path)?;
    } else {
        let walker = ignore::WalkBuilder::new(path)
            .hidden(true)
            .git_ignore(true)
            .sort_by_file_path(|a, b| a.cmp(b))
            .build();

        for entry in walker {
            let entry = entry?;
            let entry_path = entry.path();
            if !entry_path.is_file() {
                continue;
            }
            if detect_lang(entry_path).is_err() {
                continue;
            }
            let _ = print_exports(entry_path);
        }
    }

    Ok(())
}

fn print_exports(file: &Path) -> Result<()> {
    let lang = detect_lang(file)?;
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();

    let exports = match lang {
        SupportLang::Rust => collect_rust_exports(&root),
        SupportLang::Go => collect_go_exports(&root),
        SupportLang::Python => collect_python_exports(&root),
        _ => collect_ts_js_exports(&root), // TS/JS and similar
    };

    if exports.is_empty() {
        return Ok(());
    }

    let file_str = file.display().to_string();
    println!("File: {file_str}");
    for exp in &exports {
        println!("  {:>4}: {} ({})", exp.line + 1, exp.name, exp.kind);
    }
    Ok(())
}

/// TS/JS: collect symbols inside `export_statement` nodes.
fn collect_ts_js_exports(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();
    for node in root.children() {
        if node.kind().as_ref() != "export_statement" {
            continue;
        }
        for child in node.children() {
            let kind_str = child.kind().to_string();
            let (name, kind_label) = match kind_str.as_str() {
                "function_declaration" | "generator_function_declaration" => {
                    (get_name(&child), "function")
                }
                "class_declaration" => (get_name(&child), "class"),
                "interface_declaration" => (get_name(&child), "interface"),
                "type_alias_declaration" => (get_name(&child), "type"),
                "enum_declaration" => (get_name(&child), "enum"),
                "lexical_declaration" | "variable_declaration" => {
                    // const/let/var — extract from variable_declarator
                    let mut n = None;
                    for inner in child.children() {
                        if inner.kind().as_ref() == "variable_declarator" {
                            n = inner.field("name").map(|nn| nn.text().to_string());
                            break;
                        }
                    }
                    (n, "const")
                }
                _ => continue,
            };
            if let Some(name) = name {
                exports.push(ExportedSymbol {
                    name,
                    kind: kind_label.to_string(),
                    line: child.start_pos().line(),
                });
            }
        }
    }
    exports
}

/// Rust: collect items with `pub` visibility modifier.
fn collect_rust_exports(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();
    for node in root.children() {
        let kind = node.kind().to_string();
        // Check if the node has a visibility modifier that is `pub`
        let is_pub = node
            .children()
            .any(|c| c.kind().as_ref() == "visibility_modifier");

        if !is_pub {
            continue;
        }

        let kind_label = match kind.as_str() {
            "function_item" => "fn",
            "struct_item" => "struct",
            "enum_item" => "enum",
            "trait_item" => "trait",
            "impl_item" => "impl",
            "type_item" => "type",
            "const_item" => "const",
            "static_item" => "static",
            "mod_item" => "mod",
            _ => continue,
        };

        if let Some(name) = get_name(&node) {
            exports.push(ExportedSymbol {
                name,
                kind: kind_label.to_string(),
                line: node.start_pos().line(),
            });
        }
    }
    exports
}

/// Go: collect top-level declarations with uppercase first letter.
fn collect_go_exports(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();
    for node in root.children() {
        let kind = node.kind().to_string();
        let kind_label = match kind.as_str() {
            "function_declaration" => "func",
            "method_declaration" => "method",
            "type_declaration" => "type",
            _ => continue,
        };
        if let Some(name) = get_name(&node) {
            if name.starts_with(|c: char| c.is_uppercase()) {
                exports.push(ExportedSymbol {
                    name,
                    kind: kind_label.to_string(),
                    line: node.start_pos().line(),
                });
            }
        }
    }
    exports
}

/// Python: collect top-level definitions (no leading underscore).
fn collect_python_exports(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Vec<ExportedSymbol> {
    let mut exports = Vec::new();
    for node in root.children() {
        let kind = node.kind().to_string();
        let kind_label = match kind.as_str() {
            "function_definition" => "def",
            "class_definition" => "class",
            _ => continue,
        };
        if let Some(name) = get_name(&node) {
            if !name.starts_with('_') {
                exports.push(ExportedSymbol {
                    name,
                    kind: kind_label.to_string(),
                    line: node.start_pos().line(),
                });
            }
        }
    }
    exports
}

fn get_name(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Option<String> {
    node.field("name").map(|n| n.text().to_string())
}
