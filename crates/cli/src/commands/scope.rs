use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, parse_file};

/// Node kinds representing function/method/closure scopes.
const SCOPE_KINDS: &[&str] = &[
    "function_declaration",
    "function_definition",
    "function_item", // Rust
    "arrow_function",
    "method_definition",
    "method_declaration",
    "function_signature_item",
    "generator_function_declaration",
    "class_declaration",
    "class_definition",
    "impl_item",
    "trait_item",
    "closure_expression", // Rust
    // Python
    "lambda",
];

/// Main entry: if target is numeric → line-based scope lookup,
/// if target is a symbol name → show children of that symbol.
pub fn run(file: &Path, target: &str) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File does not exist: {}", file.display());
    }

    // Detect: is this a line number or a symbol name?
    if target.chars().all(|c| c.is_ascii_digit()) {
        let line: usize = target.parse().unwrap_or(1);
        run_line_scope(file, line)
    } else {
        run_symbol_children(file, target)
    }
}

/// Original behavior: show enclosing scopes for a given line.
fn run_line_scope(file: &Path, line: usize) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let line = line.max(1); // clamp to 1-indexed minimum
    let lines: Vec<&str> = source.lines().collect();
    // Convert to 0-based and clamp to last valid line
    let target_line = line.saturating_sub(1).min(lines.len().saturating_sub(1));

    // DFS to find all scopes containing the target line
    let mut scopes: Vec<ScopeInfo> = Vec::new();
    find_scopes_at_line(&root, target_line, &mut scopes);

    let file_str = file.display().to_string();

    if scopes.is_empty() {
        println!("Scope at {file_str}:{line}");
        println!("  (top-level / module scope)");
        println!();
        println!("Context line:");
        if let Some(l) = lines.get(target_line) {
            println!("{line:>4}: {l}");
        }
        return Ok(());
    }

    println!("Scope at {file_str}:{line}");
    println!();

    for (i, scope) in scopes.iter().enumerate() {
        let indent = "  ".repeat(i);
        let first_line = scope.text.lines().next().unwrap_or("");
        println!(
            "{}{}:{} | {}",
            indent,
            humanize_kind(&scope.kind),
            scope.start_line + 1,
            first_line.trim()
        );
    }

    // Show the innermost scope's signature with more detail
    if let Some(innermost) = scopes.last() {
        println!();
        println!("Innermost scope ({}):", humanize_kind(&innermost.kind));
        // Print signature lines (first few lines of the scope)
        let sig_end = (innermost.start_line + 3).min(innermost.end_line);
        for i in innermost.start_line..=sig_end {
            if let Some(l) = lines.get(i) {
                println!("{:>4}: {}", i + 1, l);
            }
        }
        if sig_end < innermost.end_line {
            println!("      ... ({} more lines)", innermost.end_line - sig_end);
        }
    }

    Ok(())
}

/// Reverse mode: given a symbol name, show all child symbols within it.
fn run_symbol_children(file: &Path, symbol: &str) -> Result<()> {
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);
    let file_str = file.display().to_string();

    // Find the target symbol
    let parent = defs.iter().find(|d| d.name.as_deref() == Some(symbol));

    match parent {
        Some(p) => {
            println!("Children of '{}' ({}):", symbol, file_str);
            println!(
                "  {}:{}-{} | {}",
                symbol,
                p.start_line + 1,
                p.end_line + 1,
                p.text.lines().next().unwrap_or("").trim()
            );
            println!();

            // Find all definitions within the parent's range with greater depth
            let children: Vec<_> = defs
                .iter()
                .filter(|d| {
                    d.start_line >= p.start_line
                        && d.end_line <= p.end_line
                        && d.depth > p.depth
                        && d.name.is_some()
                })
                .collect();

            if children.is_empty() {
                println!("  (no child symbols)");
            } else {
                for child in &children {
                    let indent = "  ".repeat(child.depth - p.depth);
                    let name = child.name.as_deref().unwrap_or("?");
                    let kind_label = if child.is_field { "field" } else { humanize_kind(&child.kind) };
                    let first_line = child.text.lines().next().unwrap_or("").trim();
                    println!(
                        "  {}{} ({}) :{} | {}",
                        indent,
                        name,
                        kind_label,
                        child.start_line + 1,
                        first_line
                    );
                }
            }
        }
        None => {
            // Try fuzzy match
            let names: Vec<&str> = defs
                .iter()
                .filter_map(|d| d.name.as_deref())
                .collect();
            let mut msg = format!("Symbol '{}' not found in {}", symbol, file_str);
            let similar: Vec<&&str> = names
                .iter()
                .filter(|n| n.contains(symbol) || symbol.contains(**n))
                .collect();
            if !similar.is_empty() {
                msg.push_str(". Similar:");
                for s in similar.iter().take(5) {
                    msg.push_str(&format!(" {s}"));
                }
            }
            anyhow::bail!("{msg}");
        }
    }

    Ok(())
}

/// Map raw tree-sitter AST node kind to a human-readable label.
fn humanize_kind(kind: &str) -> &str {
    match kind {
        "function_declaration" | "function_definition" | "function_item" => "fn",
        "method_definition" | "method_declaration" => "method",
        "function_signature_item" => "trait fn",
        "arrow_function" => "arrow fn",
        "generator_function_declaration" => "generator fn",
        "class_declaration" | "class_definition" => "class",
        "impl_item" => "impl",
        "trait_item" => "trait",
        "closure_expression" => "closure",
        "lambda" => "lambda",
        _ => kind,
    }
}

struct ScopeInfo {
    kind: String,
    start_line: usize,
    end_line: usize,
    text: String,
}

fn find_scopes_at_line(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    target_line: usize,
    scopes: &mut Vec<ScopeInfo>,
) {
    let start = node.start_pos().line();
    let end = node.end_pos().line();

    if target_line < start || target_line > end {
        return;
    }

    let kind = node.kind().to_string();
    if SCOPE_KINDS.contains(&kind.as_str()) {
        scopes.push(ScopeInfo {
            kind: kind.clone(),
            start_line: start,
            end_line: end,
            text: node.text().to_string(),
        });
    }

    for child in node.children() {
        find_scopes_at_line(&child, target_line, scopes);
    }
}
