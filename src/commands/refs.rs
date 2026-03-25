use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{detect_lang, find_references_in_node, parse_file};
use crate::formatter;

pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut all_refs: Vec<(String, usize, String)> = Vec::new();

    if search_dir.is_file() {
        collect_refs_in_file(search_dir, symbol, &mut all_refs)?;
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
            let _ = collect_refs_in_file(entry_path, symbol, &mut all_refs);
        }
    }

    let output = formatter::format_refs_output(symbol, &all_refs);
    print!("{}", output);
    Ok(())
}

fn collect_refs_in_file(
    file: &Path,
    symbol: &str,
    all_refs: &mut Vec<(String, usize, String)>,
) -> Result<()> {
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let refs = find_references_in_node(&root, symbol, &source);
    let file_str = file.display().to_string();

    for r in refs {
        all_refs.push((file_str.clone(), r.line, r.line_text));
    }

    Ok(())
}
