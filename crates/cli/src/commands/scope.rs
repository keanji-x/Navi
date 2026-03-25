use anyhow::Result;
use std::path::Path;

use crate::ast::engine::parse_file;

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

pub fn run(file: &Path, line: usize) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File does not exist: {}", file.display());
    }

    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let lines: Vec<&str> = source.lines().collect();
    // Convert to 0-based and clamp to last valid line (editors may report cursor at EOF)
    let target_line = line.saturating_sub(1).min(lines.len().saturating_sub(1));

    // DFS to find all scopes containing the target line, from outermost to innermost
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
            scope.kind,
            scope.start_line + 1,
            first_line.trim()
        );
    }

    // Show the innermost scope's signature with more detail
    if let Some(innermost) = scopes.last() {
        println!();
        println!("Innermost scope ({}):", innermost.kind);
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
