use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};

pub fn run(symbol: &str, path: Option<&Path>, max_depth: usize) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut found = false;
    expand_type(symbol, search_dir, max_depth, 0, &mut visited, &mut found)?;

    if !found {
        println!("No type definition found for '{symbol}'");
    }

    Ok(())
}

fn expand_type(
    symbol: &str,
    search_dir: &Path,
    max_depth: usize,
    current_depth: usize,
    visited: &mut HashSet<String>,
    found: &mut bool,
) -> Result<()> {
    if visited.contains(symbol) {
        return Ok(());
    }
    visited.insert(symbol.to_string());

    if search_dir.is_file() {
        if detect_lang(search_dir).is_ok() {
            process_type_file(
                search_dir,
                symbol,
                search_dir,
                max_depth,
                current_depth,
                visited,
                found,
            )?;
        }
    } else {
        let walker = ignore::WalkBuilder::new(search_dir)
            .hidden(true)
            .git_ignore(true)
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
            process_type_file(
                entry_path,
                symbol,
                search_dir,
                max_depth,
                current_depth,
                visited,
                found,
            )?;
        }
    }

    Ok(())
}

fn process_type_file(
    entry_path: &Path,
    symbol: &str,
    search_dir: &Path,
    max_depth: usize,
    current_depth: usize,
    visited: &mut HashSet<String>,
    found: &mut bool,
) -> Result<()> {
    let (grep, source) = parse_file(entry_path)?;
    let root = grep.root();
    let defs = collect_definitions(&root);
    let indent = "  ".repeat(current_depth);

    for def in &defs {
        if let Some(ref name) = def.name {
            if name == symbol {
                *found = true;
                let file_str = entry_path.display().to_string();
                let lines: Vec<&str> = source.lines().collect();

                if current_depth == 0 {
                    println!(
                        "{}# {} ({}:{})",
                        indent,
                        symbol,
                        file_str,
                        def.start_line + 1
                    );
                } else {
                    println!(
                        "{}→ {} ({}:{})",
                        indent,
                        symbol,
                        file_str,
                        def.start_line + 1
                    );
                }

                // Print the definition body
                for i in def.start_line..=def.end_line {
                    if let Some(line) = lines.get(i) {
                        println!("{}{:>4}: {}", indent, i + 1, line);
                    }
                }
                println!();

                // Extract type references from the definition body if we can go deeper
                if current_depth < max_depth {
                    let type_refs =
                        extract_type_identifiers_from_range(&root, def.start_line, def.end_line);
                    for type_ref in type_refs {
                        if !visited.contains(&type_ref) && type_ref != symbol {
                            let _ = expand_type(
                                &type_ref,
                                search_dir,
                                max_depth,
                                current_depth + 1,
                                visited,
                                found,
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Extract all type_identifier nodes within a line range of the AST.
fn extract_type_identifiers_from_range(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    start_line: usize,
    end_line: usize,
) -> Vec<String> {
    let mut types = HashSet::new();

    for n in root.dfs() {
        let kind = n.kind();
        if kind.as_ref() == "type_identifier" || kind.as_ref() == "type_annotation" {
            let line = n.start_pos().line();
            if line >= start_line && line <= end_line {
                let text = n.text().to_string();
                // Skip built-in types
                if !is_builtin_type(&text) {
                    // For type annotations, try to get the inner type
                    if kind.as_ref() == "type_identifier" {
                        types.insert(text);
                    }
                }
            }
        }
    }

    let mut result: Vec<String> = types.into_iter().collect();
    result.sort();
    result
}

fn is_builtin_type(t: &str) -> bool {
    matches!(
        t,
        "string"
            | "number"
            | "boolean"
            | "void"
            | "null"
            | "undefined"
            | "any"
            | "never"
            | "unknown"
            | "object"
            | "symbol"
            | "bigint"
            | "String"
            | "Number"
            | "Boolean"
            | "Object"
            | "Array"
            | "Map"
            | "Set"
            | "Promise"
            | "Record"
            | "Partial"
            | "Required"
            | "Readonly"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "f32"
            | "f64"
            | "bool"
            | "char"
            | "str"
            | "Vec"
            | "Box"
            | "Option"
            | "Result"
            | "Self"
            | "int"
            | "float"
            | "dict"
            | "list"
            | "tuple"
            | "set"
            | "bytes"
    )
}
