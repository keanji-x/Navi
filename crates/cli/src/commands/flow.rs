use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::ast::engine::{
    collect_definitions, detect_lang, find_callers_in_node, find_callees_named_in_range,
    find_references_in_node, parse_file,
};

/// A node in the call chain.
struct FlowNode {
    name: String,
    file: String,
    line: usize, // 0-based
    depth: usize,
}

pub fn run(symbol: &str, path: Option<&Path>, max_depth: usize, down: bool) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    println!("{symbol}");

    if down {
        // Callee chain: trace what this function calls
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(symbol.to_string());

        let mut chain: Vec<FlowNode> = Vec::new();
        expand_callees(symbol, search_dir, 1, max_depth, &mut visited, &mut chain)?;

        if chain.is_empty() {
            println!("  (no callees found)");
        } else {
            for node in &chain {
                let indent = "  ".repeat(node.depth);
                println!(
                    "{}→ {} ({}:{})",
                    indent,
                    node.name,
                    node.file,
                    node.line + 1
                );
            }
        }
    } else {
        // Caller chain: trace who calls this function (existing behavior)
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(symbol.to_string());

        let mut chain: Vec<FlowNode> = Vec::new();
        expand_callers(symbol, search_dir, 1, max_depth, &mut visited, &mut chain)?;

        if chain.is_empty() {
            // P2: Check for indirect references when no direct callers found
            let indirect = find_indirect_references(symbol, search_dir)?;
            if indirect.is_empty() {
                println!("  (no callers found)");
            } else {
                println!(
                    "  (no direct callers — {} indirect reference{} found, may be passed as callback/value)",
                    indirect.len(),
                    if indirect.len() == 1 { "" } else { "s" }
                );
                for r in &indirect {
                    let ctx = match &r.enclosing_fn {
                        Some(fn_name) => format!(" (in {fn_name})"),
                        None => String::new(),
                    };
                    println!("    ~ {}:{} | {}{}", r.file, r.line + 1, r.line_text.trim(), ctx);
                }
                // Actionable hints: suggest tracing the enclosing functions
                let fn_names: Vec<&str> = indirect
                    .iter()
                    .filter_map(|r| r.enclosing_fn.as_deref())
                    .collect::<HashSet<_>>()
                    .into_iter()
                    .collect();
                if !fn_names.is_empty() {
                    let suggestions: Vec<String> = fn_names.iter()
                        .take(3)
                        .map(|n| format!("navi flow {n}"))
                        .collect();
                    println!("  hint: try `{}` to trace the registration path",
                             suggestions.join("` or `"));
                }
            }
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

fn expand_callees(
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

    let callee_names = find_callee_function_names(symbol, search_dir)?;

    for (callee_name, file, line) in callee_names {
        if visited.contains(&callee_name) {
            chain.push(FlowNode {
                name: format!("{callee_name} (cycle)"),
                file,
                line,
                depth: current_depth,
            });
            continue;
        }

        chain.push(FlowNode {
            name: callee_name.clone(),
            file: file.clone(),
            line,
            depth: current_depth,
        });

        visited.insert(callee_name.clone());
        expand_callees(
            &callee_name,
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

/// For a given symbol, find all functions it calls inside its body.
fn find_callee_function_names(
    symbol: &str,
    search_dir: &Path,
) -> Result<Vec<(String, String, usize)>> {
    let mut results: Vec<(String, String, usize)> = Vec::new();

    let walk_entries = |dir: &Path| -> Result<Vec<std::path::PathBuf>> {
        let mut files = Vec::new();
        let walker = ignore::WalkBuilder::new(dir)
            .hidden(true)
            .git_ignore(true)
            .build();
        for entry in walker {
            let entry = entry?;
            let p = entry.path().to_path_buf();
            if p.is_file() && detect_lang(&p).is_ok() {
                files.push(p);
            }
        }
        Ok(files)
    };

    let files = if search_dir.is_file() {
        vec![search_dir.to_path_buf()]
    } else {
        walk_entries(search_dir)?
    };

    // Find the definition of the symbol to get its body range
    for file_path in &files {
        if let Ok((grep, source)) = parse_file(file_path) {
            let root = grep.root();
            let defs = collect_definitions(&root);

            for def in &defs {
                if def.name.as_deref() == Some(symbol) {
                    // Found the function — scan its body for call expressions
                    let callees = find_callees_named_in_range(&root, def.start_line, def.end_line, &source);
                    let file_str = file_path.display().to_string();

                    for c in &callees {
                        if c.name != symbol {
                            results.push((c.name.clone(), file_str.clone(), c.line));
                        }
                    }
                    return deduplicate(results);
                }
            }
        }
    }

    deduplicate(results)
}

fn deduplicate(mut results: Vec<(String, String, usize)>) -> Result<Vec<(String, String, usize)>> {
    let mut seen = HashSet::new();
    results.retain(|(name, _, _)| seen.insert(name.clone()));
    Ok(results)
}

/// Check if a reference line is noise (import, definition, type annotation).
fn is_noise_reference(trimmed: &str, symbol: &str) -> bool {
    // Skip import lines
    if trimmed.starts_with("import ")
        || trimmed.starts_with("from ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("require(")
    {
        return true;
    }
    // Skip pure type annotation lines (heuristic)
    if trimmed.starts_with("type ")
        || trimmed.starts_with("interface ")
    {
        return true;
    }
    // Skip definition lines (function X, const X, etc.)
    let def_prefixes = [
        format!("function {symbol}"),
        format!("const {symbol}"),
        format!("let {symbol}"),
        format!("var {symbol}"),
        format!("export function {symbol}"),
        format!("export const {symbol}"),
        format!("export default function {symbol}"),
        format!("export async function {symbol}"),
        format!("async function {symbol}"),
        format!("fn {symbol}"),
        format!("pub fn {symbol}"),
        format!("pub(crate) fn {symbol}"),
        format!("def {symbol}"),
        format!("func {symbol}"),
        format!("class {symbol}"),
        format!("export class {symbol}"),
    ];
    for prefix in &def_prefixes {
        if trimmed.starts_with(prefix.as_str()) {
            return true;
        }
    }
    false
}

/// Indirect reference with context about where the symbol is used.
struct IndirectRef {
    file: String,
    line: usize,
    line_text: String,
    enclosing_fn: Option<String>,
}

/// P2: Find references to a symbol that are NOT call-sites (indirect usage).
/// These indicate the symbol is passed as a callback, stored in a variable, etc.
fn find_indirect_references(
    symbol: &str,
    search_dir: &Path,
) -> Result<Vec<IndirectRef>> {
    let mut indirect: Vec<IndirectRef> = Vec::new();

    let process_file = |file_path: &Path, out: &mut Vec<IndirectRef>| -> Result<()> {
        if let Ok((grep, source)) = parse_file(file_path) {
            let root = grep.root();
            let all_refs = find_references_in_node(&root, symbol, &source);
            let call_refs = find_callers_in_node(&root, symbol, &source);
            let call_lines: HashSet<usize> = call_refs.iter().map(|r| r.line).collect();
            let file_str = file_path.display().to_string();
            let defs = collect_definitions(&root);

            for r in &all_refs {
                if call_lines.contains(&r.line) {
                    continue;
                }
                if is_noise_reference(r.line_text.trim(), symbol) {
                    continue;
                }
                let enclosing_fn = find_enclosing_function(&defs, r.line);
                out.push(IndirectRef {
                    file: file_str.clone(),
                    line: r.line,
                    line_text: r.line_text.clone(),
                    enclosing_fn,
                });
            }
        }
        Ok(())
    };

    if search_dir.is_file() {
        process_file(search_dir, &mut indirect)?;
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
            process_file(entry_path, &mut indirect)?;
        }
    }

    Ok(indirect)
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
