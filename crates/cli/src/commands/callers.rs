use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{detect_lang, find_callers_in_node, parse_file};

pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut all_callers: Vec<(String, usize, String)> = Vec::new();

    if search_dir.is_file() {
        collect_callers_in_file(search_dir, symbol, &mut all_callers)?;
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
            let _ = collect_callers_in_file(entry_path, symbol, &mut all_callers);
        }
    }

    if all_callers.is_empty() {
        println!("No callers found for '{symbol}'");
    } else {
        println!("Found {} callers for '{symbol}':", all_callers.len());
        for (file, line, text) in &all_callers {
            println!("- {}: {} | {}", file, line + 1, text.trim());
        }
    }
    Ok(())
}

fn collect_callers_in_file(
    file: &Path,
    symbol: &str,
    all_callers: &mut Vec<(String, usize, String)>,
) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let callers = find_callers_in_node(&root, symbol, &source);
    let file_str = file.display().to_string();

    for c in callers {
        all_callers.push((file_str.clone(), c.line, c.line_text));
    }

    Ok(())
}
