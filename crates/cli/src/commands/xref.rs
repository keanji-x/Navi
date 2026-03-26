use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{
    collect_definitions, detect_lang, find_callers_in_node, find_references_in_node, parse_file,
};

pub fn run(symbol: &str, path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let mut definitions: Vec<(String, usize, usize)> = Vec::new(); // (file, start, end)
    let mut callers: Vec<(String, usize, String)> = Vec::new(); // (file, line, text)
    let mut references: Vec<(String, usize, String)> = Vec::new(); // (file, line, text)

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
        if let Ok((grep, source)) = parse_file(entry_path) {
            let root = grep.root();
            let file_str = entry_path.display().to_string();

            // Collect definitions
            let defs = collect_definitions(&root);
            for def in &defs {
                if def.name.as_deref() == Some(symbol) {
                    definitions.push((file_str.clone(), def.start_line, def.end_line));
                }
            }

            // Collect callers
            let file_callers = find_callers_in_node(&root, symbol, &source);
            for c in &file_callers {
                callers.push((file_str.clone(), c.line, c.line_text.clone()));
            }

            // Collect references (all identifier occurrences)
            let file_refs = find_references_in_node(&root, symbol, &source);
            for r in &file_refs {
                references.push((file_str.clone(), r.line, r.line_text.clone()));
            }
        }
    }

    println!("Cross-references for '{symbol}':");
    println!();

    // --- Definitions ---
    if definitions.is_empty() {
        println!("Definitions: (none)");
    } else {
        println!("Definitions ({}):", definitions.len());
        for (file, start, end) in &definitions {
            println!("  {}:{}-{}", file, start + 1, end + 1);
        }
    }
    println!();

    // --- Callers ---
    if callers.is_empty() {
        println!("Callers: (none)");
    } else {
        println!("Callers ({}):", callers.len());
        for (file, line, text) in &callers {
            println!("  {}:{} | {}", file, line + 1, text.trim());
        }
    }
    println!();

    // --- References ---
    // Deduplicate references that are already shown as callers or definitions
    let ref_count = references.len();
    if ref_count == 0 {
        println!("References: (none)");
    } else {
        println!("All references ({}):", ref_count);
        for (file, line, text) in &references {
            println!("  {}:{} | {}", file, line + 1, text.trim());
        }
    }

    Ok(())
}
