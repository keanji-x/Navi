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

    if search_dir.is_file() {
        collect_matches_in_file(search_dir, symbol, &mut matches)?;
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
            let _ = collect_matches_in_file(entry_path, symbol, &mut matches);
        }
    }

    if matches.is_empty() {
        println!("No results found for '{symbol}'");
        return Ok(());
    }

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

fn collect_matches_in_file(file: &Path, symbol: &str, matches: &mut Vec<MatchInfo>) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);
    let file_str = file.display().to_string();

    for def in &defs {
        if let Some(ref name) = def.name {
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
