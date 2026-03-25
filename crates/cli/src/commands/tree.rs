use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};
use crate::formatter;

pub fn run(path: Option<&Path>, max_depth: Option<usize>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    if search_dir.is_file() {
        return print_file_skeleton(search_dir);
    }

    let mut builder = ignore::WalkBuilder::new(search_dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .sort_by_file_path(|a, b| a.cmp(b));
    if let Some(d) = max_depth {
        builder.max_depth(Some(d));
    }
    let walker = builder.build();

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        if detect_lang(entry_path).is_err() {
            continue;
        }
        let _ = print_file_skeleton(entry_path);
    }

    Ok(())
}

fn print_file_skeleton(file: &Path) -> Result<()> {
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);

    if defs.is_empty() {
        return Ok(());
    }

    let file_str = file.display().to_string();
    println!("File: {file_str}");
    for def in &defs {
        let skeleton = formatter::format_skeleton_line(&def.text, def.start_line, def.depth);
        println!("{skeleton}");
    }

    Ok(())
}
