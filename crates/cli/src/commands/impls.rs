use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};

/// Find all types that implement a given trait/interface.
///
/// Strategy: walk all source files and look for `impl_item` / `class_declaration`
/// nodes whose AST contains the target trait/interface name in the right position.
pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut results: Vec<ImplResult> = Vec::new();

    if search_dir.is_file() {
        if detect_lang(search_dir).is_ok() {
            collect_impls_in_file(search_dir, symbol, &mut results)?;
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
            let _ = collect_impls_in_file(entry_path, symbol, &mut results);
        }
    }

    if results.is_empty() {
        println!("No implementations found for '{symbol}'");
    } else {
        println!("Found {} implementations for '{symbol}':", results.len());
        for r in &results {
            println!();
            println!("  {} ({}:{})", r.implementor, r.file, r.start_line + 1);
            // Print a compact skeleton of the impl block
            for (i, line) in r.body_lines.iter().enumerate() {
                let line_no = r.start_line + 1 + i;
                println!("  {:>4}: {}", line_no, line);
            }
        }
    }
    Ok(())
}

struct ImplResult {
    file: String,
    implementor: String, // e.g. "impl Trait for Type" or "class Foo implements Bar"
    start_line: usize,   // 0-based
    body_lines: Vec<String>,
}

fn collect_impls_in_file(
    file: &Path,
    symbol: &str,
    results: &mut Vec<ImplResult>,
) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let lines: Vec<&str> = source.lines().collect();
    let file_str = file.display().to_string();

    // Strategy: walk the AST looking for impl_item, class_declaration, etc.
    // and check if the target trait/interface name appears as a child.
    for node in root.dfs() {
        let kind = node.kind().to_string();
        match kind.as_str() {
            // Rust: `impl Trait for Type { ... }`
            "impl_item" => {
                // Check if this impl block references the target trait
                // Look for a `type_identifier` child matching the symbol
                let _text = node.text().to_string();
                if !contains_trait_impl(&node, symbol) {
                    continue;
                }

                let start = node.start_pos().line();
                let end = node.end_pos().line();
                let header = build_impl_header(&lines, start, end);
                let body = collect_skeleton_lines(&lines, start, end);

                results.push(ImplResult {
                    file: file_str.clone(),
                    implementor: header,
                    start_line: start,
                    body_lines: body,
                });
            }
            // TS/JS/Java/etc: `class Foo implements Bar`
            "class_declaration" | "class_definition" => {
                if !contains_interface_impl(&node, symbol) {
                    continue;
                }

                let start = node.start_pos().line();
                let end = node.end_pos().line();
                let header = build_impl_header(&lines, start, end);
                let body = collect_skeleton_lines(&lines, start, end);

                results.push(ImplResult {
                    file: file_str.clone(),
                    implementor: header,
                    start_line: start,
                    body_lines: body,
                });
            }
            _ => {}
        }
    }

    // Also check definitions for trait/interface that match the symbol itself
    // to show the original definition
    let defs = collect_definitions(&root);
    for def in &defs {
        if let Some(ref name) = def.name {
            if name == symbol
                && (def.kind == "trait_item"
                    || def.kind == "interface_declaration"
                    || def.kind == "type_alias_declaration")
            {
                // Already covered by the impl search above; skip the definition itself
            }
        }
    }

    Ok(())
}

/// Check if a Rust `impl_item` node contains a trait reference matching the symbol.
/// Handles both `impl Trait for Type` and generic forms.
fn contains_trait_impl(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    symbol: &str,
) -> bool {
    // In Rust tree-sitter, impl_item children include:
    // - type_identifier nodes for the trait and the type
    // - "for" keyword
    // We look for a type_identifier matching symbol that appears BEFORE "for"
    let mut found_symbol = false;

    for child in node.children() {
        let ck = child.kind().to_string();
        if ck == "type_identifier" || ck == "scoped_type_identifier" || ck == "generic_type" {
            let text = if ck == "type_identifier" {
                child.text().to_string()
            } else {
                // For scoped/generic, extract the base type name
                extract_base_type_name(&child)
            };
            if text == symbol {
                found_symbol = true;
            }
        }
        if child.text().as_ref() == "for" && found_symbol {
            return true; // symbol appeared before "for" → it's the trait
        }
    }

    // If there's no "for" keyword, this is `impl Type { ... }` (inherent impl)
    // — we only want trait implementations
    false
}

/// Check if a class declaration implements an interface matching the symbol.
fn contains_interface_impl(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    symbol: &str,
) -> bool {
    // Look for "implements" clause or extends clause containing the symbol
    for child in node.dfs() {
        let ck = child.kind().to_string();
        // TS/JS: `implements_clause`, Java: `super_interfaces`
        if ck == "implements_clause"
            || ck == "super_interfaces"
            || ck == "extends_clause"
            || ck == "constraint"
        {
            // Check if any type_identifier child matches
            for inner in child.dfs() {
                let ik = inner.kind().to_string();
                if (ik == "type_identifier" || ik == "identifier") && inner.text().as_ref() == symbol
                {
                    return true;
                }
            }
        }
    }
    false
}

/// Extract the base type name from a scoped_type_identifier or generic_type node.
fn extract_base_type_name(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
) -> String {
    // For generic_type like `Iterator<Item = T>`, get "Iterator"
    // For scoped_type_identifier like `std::fmt::Display`, get "Display"
    for child in node.children() {
        let ck = child.kind().to_string();
        if ck == "type_identifier" {
            return child.text().to_string();
        }
    }
    // Fallback: try the last segment of scoped identifiers
    let text = node.text().to_string();
    text.split("::").last().unwrap_or(&text).to_string()
}

/// Build a one-line header for the impl block from source lines.
fn build_impl_header(lines: &[&str], start: usize, _end: usize) -> String {
    if let Some(line) = lines.get(start) {
        let trimmed = line.trim();
        // Take up to the opening brace
        if let Some(pos) = trimmed.find('{') {
            trimmed[..pos].trim().to_string()
        } else {
            trimmed.to_string()
        }
    } else {
        String::from("<unknown>")
    }
}

/// Collect a skeleton view of the impl block: first line, method signatures, last line.
fn collect_skeleton_lines(lines: &[&str], start: usize, end: usize) -> Vec<String> {
    let mut result = Vec::new();
    let max_lines = 30; // Cap output to avoid flooding

    if end - start + 1 <= max_lines {
        // Small enough to show in full
        for i in start..=end {
            if let Some(line) = lines.get(i) {
                result.push(line.to_string());
            }
        }
    } else {
        // Show first line (header) + method signatures + last line
        if let Some(line) = lines.get(start) {
            result.push(line.to_string());
        }
        // Scan for fn/method signatures inside the block
        for i in (start + 1)..end {
            if let Some(line) = lines.get(i) {
                let trimmed = line.trim();
                if trimmed.starts_with("fn ")
                    || trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("async fn ")
                    || trimmed.starts_with("pub async fn ")
                    || trimmed.starts_with("type ")
                    || trimmed.starts_with("const ")
                {
                    result.push(line.to_string());
                }
            }
        }
        if result.len() == 1 {
            // No fn signatures found, show a truncated view
            result.push(format!("    // ... {} lines omitted", end - start - 1));
        }
        if let Some(line) = lines.get(end) {
            result.push(line.to_string());
        }
    }
    result
}
