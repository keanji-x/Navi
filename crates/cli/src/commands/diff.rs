use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};

pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    // Step 1: Find the symbol's location(s) in the current source
    let mut symbol_ranges: Vec<(String, usize, usize)> = Vec::new(); // (file, start, end) 0-based

    if search_dir.is_file() {
        find_symbol_in_file(search_dir, symbol, &mut symbol_ranges)?;
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
            let _ = find_symbol_in_file(entry_path, symbol, &mut symbol_ranges);
        }
    }

    if symbol_ranges.is_empty() {
        println!("No definition found for '{symbol}'");
        return Ok(());
    }

    // Step 2: For each file containing the symbol, get git diff and filter hunks
    let mut found_any_diff = false;

    for (file, sym_start, sym_end) in &symbol_ranges {
        let diff_output = get_git_diff(file)?;
        if diff_output.is_empty() {
            continue;
        }

        // Parse and filter hunks that overlap with the symbol's line range
        let filtered = filter_hunks_for_range(&diff_output, *sym_start, *sym_end);
        if !filtered.is_empty() {
            if found_any_diff {
                println!();
            }
            println!("diff for '{symbol}' in {file}:");
            print!("{filtered}");
            found_any_diff = true;
        }
    }

    if !found_any_diff {
        println!("No changes found for '{symbol}' in git diff");
    }

    Ok(())
}

/// `navi diff --since N` — summarize all symbols changed in last N commits.
pub fn run_since(n: usize, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    // Get list of files changed in last N commits
    let output = Command::new("git")
        .args(["log", &format!("-{n}"), "--name-only", "--pretty=format:"])
        .current_dir(search_dir)
        .output()
        .with_context(|| "Failed to run git log")?;

    let file_list = String::from_utf8_lossy(&output.stdout);
    let mut changed_files: Vec<String> = file_list
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();
    changed_files.sort();
    changed_files.dedup();

    if changed_files.is_empty() {
        println!("No files changed in last {n} commits");
        return Ok(());
    }

    // Get the diff for all these commits
    let diff_ref = format!("HEAD~{n}");

    let mut found_any = false;

    for file in &changed_files {
        let file_path = if search_dir == Path::new(".") {
            std::path::PathBuf::from(file)
        } else {
            search_dir.join(file)
        };

        if !file_path.exists() {
            continue; // deleted file
        }
        if detect_lang(&file_path).is_err() {
            continue;
        }

        // Get diff for this file over the commit range
        let diff_output = Command::new("git")
            .args(["diff", &diff_ref, "HEAD", "--unified=0", "--", file])
            .current_dir(search_dir)
            .output()
            .with_context(|| format!("Failed to run git diff for {file}"))?;

        let diff_str = String::from_utf8_lossy(&diff_output.stdout).to_string();
        if diff_str.is_empty() {
            continue;
        }

        // Parse the AST of the current file to map lines to symbols
        if let Ok((grep, _source)) = parse_file(&file_path) {
            let root = grep.root();
            let defs = collect_definitions(&root);

            // Parse hunk headers to get changed line ranges
            let changed_lines = extract_changed_new_lines(&diff_str);

            // Map changed lines to symbols
            let mut changed_symbols: Vec<(&str, usize)> = Vec::new(); // (name, line)
            for line in &changed_lines {
                for def in &defs {
                    if def.start_line <= *line
                        && *line <= def.end_line
                        && def.name.is_some()
                    {
                        let name = def.name.as_deref().unwrap();
                        if !changed_symbols.iter().any(|(n, _)| *n == name) {
                            changed_symbols.push((name, def.start_line));
                        }
                    }
                }
            }

            if !changed_symbols.is_empty() {
                if found_any {
                    println!();
                }
                println!("{}:", file);
                for (name, line) in &changed_symbols {
                    println!("  {:>4}: {}", line + 1, name);
                }
                found_any = true;
            }
        }
    }

    if !found_any {
        println!("No symbol-level changes found in last {n} commits");
    }

    Ok(())
}

/// Extract the new-side line numbers that were changed from a unified diff.
fn extract_changed_new_lines(diff: &str) -> Vec<usize> {
    let mut lines = Vec::new();
    for line in diff.lines() {
        if line.starts_with("@@") {
            if let Some((start, count)) = parse_hunk_header(line) {
                let start_0 = start.saturating_sub(1);
                for i in 0..count {
                    lines.push(start_0 + i);
                }
            }
        }
    }
    lines
}

fn find_symbol_in_file(
    file: &Path,
    symbol: &str,
    ranges: &mut Vec<(String, usize, usize)>,
) -> Result<()> {
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);
    let file_str = file.display().to_string();

    for def in &defs {
        if let Some(ref name) = def.name {
            if name == symbol {
                ranges.push((file_str.clone(), def.start_line, def.end_line));
            }
        }
    }

    Ok(())
}

fn get_git_diff(file: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["diff", "HEAD", "--unified=3", "--", file])
        .output()
        .with_context(|| "Failed to run git diff")?;

    // Also try staged diff if HEAD diff is empty (e.g. all changes are staged)
    let diff_str = String::from_utf8_lossy(&output.stdout).to_string();
    if diff_str.is_empty() {
        let output2 = Command::new("git")
            .args(["diff", "--cached", "--unified=3", "--", file])
            .output()
            .with_context(|| "Failed to run git diff --cached")?;
        return Ok(String::from_utf8_lossy(&output2.stdout).to_string());
    }
    Ok(diff_str)
}

/// Parse unified diff and filter to only hunks overlapping [sym_start, sym_end] (0-based).
fn filter_hunks_for_range(diff: &str, sym_start: usize, sym_end: usize) -> String {
    let mut result = String::new();
    let mut current_hunk = String::new();
    let mut hunk_new_start: usize = 0;
    let mut hunk_new_end: usize = 0;
    let mut in_hunk = false;
    let mut _current_new_line: usize = 0;

    for line in diff.lines() {
        if line.starts_with("@@") {
            // Flush previous hunk if it overlaps
            if in_hunk && ranges_overlap(hunk_new_start, hunk_new_end, sym_start, sym_end) {
                result.push_str(&current_hunk);
                result.push('\n');
            }

            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            if let Some((new_start, new_count)) = parse_hunk_header(line) {
                hunk_new_start = new_start.saturating_sub(1); // convert to 0-based
                hunk_new_end = hunk_new_start + new_count.saturating_sub(1);
                _current_new_line = hunk_new_start;
                current_hunk = format!("{line}\n");
                in_hunk = true;
            }
        } else if in_hunk {
            current_hunk.push_str(line);
            current_hunk.push('\n');

            if line.starts_with('+') || line.starts_with(' ') {
                _current_new_line += 1;
            }
        }
    }

    // Flush last hunk
    if in_hunk && ranges_overlap(hunk_new_start, hunk_new_end, sym_start, sym_end) {
        result.push_str(&current_hunk);
    }

    result
}

fn ranges_overlap(a_start: usize, a_end: usize, b_start: usize, b_end: usize) -> bool {
    a_start <= b_end && b_start <= a_end
}

fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    // @@ -old_start,old_count +new_start,new_count @@ optional context
    let plus_idx = line.find('+')?;
    let rest = &line[plus_idx + 1..];
    let end = rest.find(' ').unwrap_or(rest.len());
    let range_part = &rest[..end];

    if let Some(comma_idx) = range_part.find(',') {
        let start: usize = range_part[..comma_idx].parse().ok()?;
        let count: usize = range_part[comma_idx + 1..].parse().ok()?;
        Some((start, count))
    } else {
        let start: usize = range_part.parse().ok()?;
        Some((start, 1))
    }
}
