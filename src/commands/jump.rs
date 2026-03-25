use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};
use crate::formatter;

pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    // If it's a single file, search just that file
    if search_dir.is_file() {
        return search_file(search_dir, symbol);
    }

    // Walk directory for source files
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
        // Skip files we can't detect language for
        if detect_lang(entry_path).is_err() {
            continue;
        }
        if search_file(entry_path, symbol).is_ok() {
            return Ok(());
        }
    }

    println!("No results found for '{}'", symbol);
    Ok(())
}

fn search_file(file: &Path, symbol: &str) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);

    for def in &defs {
        if let Some(ref name) = def.name {
            if name == symbol {
                let output = formatter::format_jump_output(
                    &file.display().to_string(),
                    symbol,
                    &source,
                    def.start_line,
                    def.end_line,
                    3, // context lines
                );
                print!("{}", output);
                // Return Ok to signal we found it
                return Ok(());
            }
        }
    }

    // Return error to signal "not found in this file" — not a real error
    anyhow::bail!("not found")
}
