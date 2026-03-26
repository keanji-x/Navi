use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};
use crate::formatter;

/// A matched definition location.
struct MatchInfo {
    file: String,
    start_line: usize,
    end_line: usize,
    source: String,
}

pub fn run(symbol: &str, path: Option<&Path>, show_all: bool) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut matches: Vec<MatchInfo> = Vec::new();
    let mut all_names: Vec<String> = Vec::new();

    if search_dir.is_file() {
        collect_matches_in_file(search_dir, symbol, &mut matches, &mut all_names)?;
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
            let _ = collect_matches_in_file(entry_path, symbol, &mut matches, &mut all_names);
        }
    }

    if matches.is_empty() {
        // Fuzzy suggestion: find similar names
        let suggestions = find_similar_names(&all_names, symbol, 5);
        if suggestions.is_empty() {
            println!("No results found for '{symbol}'");
        } else {
            println!("No results found for '{symbol}'. Did you mean:");
            for (name, _score) in &suggestions {
                println!("  - {name}");
            }
        }
        return Ok(());
    }

    // Sort by relevance: shorter path (closer to search root) first, then alphabetically
    matches.sort_by(|a, b| {
        let depth_a = a.file.matches('/').count();
        let depth_b = b.file.matches('/').count();
        depth_a.cmp(&depth_b).then_with(|| a.file.cmp(&b.file))
    });

    if show_all {
        // Print all definitions
        for m in &matches {
            let output = formatter::format_jump_output(
                &m.file,
                symbol,
                &m.source,
                m.start_line,
                m.end_line,
                3,
            );
            print!("{output}");
        }
    } else {
        // Print first definition
        let first = &matches[0];
        let output = formatter::format_jump_output(
            &first.file,
            symbol,
            &first.source,
            first.start_line,
            first.end_line,
            3,
        );
        print!("{output}");

        // Print "Also defined in" hints for remaining matches
        if matches.len() > 1 {
            println!("\nAlso defined in:");
            for m in &matches[1..] {
                println!("- {}: line {}", m.file, m.start_line + 1);
            }
        }
    }

    Ok(())
}

fn collect_matches_in_file(
    file: &Path,
    symbol: &str,
    matches: &mut Vec<MatchInfo>,
    all_names: &mut Vec<String>,
) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);
    let file_str = file.display().to_string();

    for def in &defs {
        if let Some(ref name) = def.name {
            all_names.push(name.clone());
            if name == symbol {
                matches.push(MatchInfo {
                    file: file_str.clone(),
                    start_line: def.start_line,
                    end_line: def.end_line,
                    source: source.clone(),
                });
            }
        }
    }

    Ok(())
}

/// Find definition names most similar to the query using Jaro-Winkler similarity.
fn find_similar_names(all_names: &[String], query: &str, max: usize) -> Vec<(String, f64)> {
    use std::collections::HashSet;
    use strsim::jaro_winkler;

    let mut seen = HashSet::new();
    let mut scored: Vec<(String, f64)> = all_names
        .iter()
        .filter(|n| seen.insert(n.to_string())) // deduplicate
        .map(|n| {
            let score = jaro_winkler(&n.to_lowercase(), &query.to_lowercase());
            (n.clone(), score)
        })
        .filter(|(_, score)| *score > 0.7) // threshold
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(max);
    scored
}
