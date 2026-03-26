use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, find_callers_in_node, parse_file};

/// A node in the call chain.
struct FlowNode {
    name: String,
    file: String,
    line: usize, // 0-based
    depth: usize,
}

pub fn run(symbol: &str, path: Option<&Path>, max_depth: usize) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    println!("{symbol}");

    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(symbol.to_string());

    let mut chain: Vec<FlowNode> = Vec::new();
    expand_callers(symbol, search_dir, 1, max_depth, &mut visited, &mut chain)?;

    if chain.is_empty() {
        println!("  (no callers found)");
    } else {
        for node in &chain {
            let indent = "  ".repeat(node.depth);
            println!(
                "{}← {} ({}:{})",
                indent,
                node.name,
                node.file,
                node.line + 1
            );
        }
    }

    Ok(())
}

fn expand_callers(
    symbol: &str,
    search_dir: &Path,
    current_depth: usize,
    max_depth: usize,
    visited: &mut HashSet<String>,
    chain: &mut Vec<FlowNode>,
) -> Result<()> {
    if current_depth > max_depth {
        return Ok(());
    }

    // Collect all caller sites for this symbol across the project
    let caller_names = find_caller_function_names(symbol, search_dir)?;

    for (caller_name, file, line) in caller_names {
        if visited.contains(&caller_name) {
            // Show cycle marker but don't recurse
            chain.push(FlowNode {
                name: format!("{caller_name} (cycle)"),
                file,
                line,
                depth: current_depth,
            });
            continue;
        }

        chain.push(FlowNode {
            name: caller_name.clone(),
            file: file.clone(),
            line,
            depth: current_depth,
        });

        visited.insert(caller_name.clone());
        expand_callers(
            &caller_name,
            search_dir,
            current_depth + 1,
            max_depth,
            visited,
            chain,
        )?;
    }

    Ok(())
}

/// For a given symbol, find all call-sites and return the enclosing function name + location.
fn find_caller_function_names(
    symbol: &str,
    search_dir: &Path,
) -> Result<Vec<(String, String, usize)>> {
    let mut results: Vec<(String, String, usize)> = Vec::new();

    let walk = |dir: &Path| -> Result<Vec<(String, String, usize)>> {
        let mut out = Vec::new();
        let walker = ignore::WalkBuilder::new(dir)
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

            if let Ok((grep, source)) = parse_file(entry_path) {
                let root = grep.root();
                let callers = find_callers_in_node(&root, symbol, &source);
                if callers.is_empty() {
                    continue;
                }

                let defs = collect_definitions(&root);
                let file_str = entry_path.display().to_string();

                for c in &callers {
                    // Find the enclosing function for this caller line
                    let enclosing = find_enclosing_function(&defs, c.line);
                    if let Some(name) = enclosing {
                        // Don't include self-references
                        if name != symbol {
                            out.push((name, file_str.clone(), c.line));
                        }
                    }
                }
            }
        }
        Ok(out)
    };

    if search_dir.is_file() {
        // Unlikely for flow, but handle it
        if let Ok((grep, source)) = parse_file(search_dir) {
            let root = grep.root();
            let callers = find_callers_in_node(&root, symbol, &source);
            let defs = collect_definitions(&root);
            let file_str = search_dir.display().to_string();
            for c in &callers {
                if let Some(name) = find_enclosing_function(&defs, c.line) {
                    if name != symbol {
                        results.push((name, file_str.clone(), c.line));
                    }
                }
            }
        }
    } else {
        results = walk(search_dir)?;
    }

    // Deduplicate by function name (keep first occurrence)
    let mut seen = HashSet::new();
    results.retain(|(name, _, _)| seen.insert(name.clone()));

    Ok(results)
}

/// Find the innermost function/method enclosing a given line.
fn find_enclosing_function(
    defs: &[crate::ast::engine::DefinitionInfo],
    line: usize,
) -> Option<String> {
    let mut best: Option<&crate::ast::engine::DefinitionInfo> = None;
    for def in defs {
        if def.start_line <= line && line <= def.end_line && def.name.is_some() {
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
