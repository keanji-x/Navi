use anyhow::Result;
use regex::Regex;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, find_references_by_pattern, find_references_in_node, parse_file};

/// Result item for grep: file, match line (0-based), matched line text, enclosing function name.
struct GrepHit {
    file: String,
    line: usize,       // 0-based
    line_text: String,
    enclosing: Option<String>, // enclosing function/method/class name
}

pub fn run(pattern: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    // Try to compile as regex; if it fails, fall back to exact match
    let re = Regex::new(&format!("^(?:{pattern})$"));

    let mut hits: Vec<GrepHit> = Vec::new();

    if search_dir.is_file() {
        collect_grep_in_file(search_dir, pattern, re.as_ref().ok(), &mut hits)?;
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
            let _ = collect_grep_in_file(entry_path, pattern, re.as_ref().ok(), &mut hits);
        }
    }

    if hits.is_empty() {
        println!("No matches found for '{pattern}'");
    } else {
        println!("Found {} matches for '{pattern}':", hits.len());
        for hit in &hits {
            let scope = hit
                .enclosing
                .as_deref()
                .unwrap_or("<top-level>");
            println!(
                "  {}:{} | in {} | {}",
                hit.file,
                hit.line + 1,
                scope,
                hit.line_text.trim()
            );
        }
    }
    Ok(())
}

fn collect_grep_in_file(
    file: &Path,
    pattern: &str,
    re: Option<&Regex>,
    hits: &mut Vec<GrepHit>,
) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let file_str = file.display().to_string();

    // Use regex matching if available, otherwise exact match
    let refs = match re {
        Some(regex) => find_references_by_pattern(&root, regex, &source),
        None => find_references_in_node(&root, pattern, &source),
    };
    if refs.is_empty() {
        return Ok(());
    }

    // Collect definitions to map lines → enclosing scope
    let defs = collect_definitions(&root);

    for r in refs {
        // Find the innermost enclosing definition
        let enclosing = find_enclosing_def(&defs, r.line);
        hits.push(GrepHit {
            file: file_str.clone(),
            line: r.line,
            line_text: r.line_text,
            enclosing,
        });
    }

    Ok(())
}

/// Find the innermost definition that encloses the given line (0-based).
fn find_enclosing_def(
    defs: &[crate::ast::engine::DefinitionInfo],
    line: usize,
) -> Option<String> {
    let mut best: Option<&crate::ast::engine::DefinitionInfo> = None;
    for def in defs {
        if def.start_line <= line && line <= def.end_line {
            // Prefer the narrowest (most deeply nested) enclosing definition
            if let Some(prev) = best {
                if (def.end_line - def.start_line) < (prev.end_line - prev.start_line) {
                    best = Some(def);
                }
            } else {
                best = Some(def);
            }
        }
    }
    best.and_then(|d| d.name.clone())
}
