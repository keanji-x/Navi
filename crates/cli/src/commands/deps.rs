use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{detect_lang, extract_imports, parse_file};

/// Walk up from a file to find the project root (directory containing `.git`).
/// Falls back to CWD if no `.git` is found.
fn find_project_root(start: &Path) -> std::path::PathBuf {
    let abs = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let mut dir = if abs.is_file() {
        abs.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| std::path::PathBuf::from("."))
    } else {
        abs
    };
    loop {
        if dir.join(".git").exists() {
            return dir;
        }
        match dir.parent() {
            Some(parent) if parent != dir => dir = parent.to_path_buf(),
            _ => return std::path::PathBuf::from("."),
        }
    }
}

/// Check if an import source references a given file.
/// Uses segment-boundary matching to avoid false positives with short stems.
fn import_matches_file(import_source: &str, file_stem: &str, file_name: &str) -> bool {
    if file_stem.is_empty() {
        return false;
    }
    // Strip common JS/TS extension variants from import source for matching
    // e.g. `./drama/DramaDirector.js` → `./drama/DramaDirector`
    let stripped = import_source
        .strip_suffix(".js")
        .or_else(|| import_source.strip_suffix(".ts"))
        .or_else(|| import_source.strip_suffix(".jsx"))
        .or_else(|| import_source.strip_suffix(".tsx"))
        .or_else(|| import_source.strip_suffix(".mjs"))
        .or_else(|| import_source.strip_suffix(".cjs"))
        .unwrap_or(import_source);

    // Exact match (with and without extension)
    if import_source == file_stem
        || import_source == file_name
        || stripped == file_stem
        || stripped == file_name
    {
        return true;
    }
    // Rust-style: `::stem` or `::stem::`
    if import_source.contains(&format!("::{file_stem}"))
        || import_source.contains(&format!("::{file_stem}::"))
    {
        return true;
    }
    // JS/TS/Python-style: path ends with `/stem` (with or without extension)
    if import_source.ends_with(&format!("/{file_stem}"))
        || import_source.ends_with(&format!("/{file_name}"))
        || stripped.ends_with(&format!("/{file_stem}"))
    {
        return true;
    }
    false
}

pub fn run(file: &Path) -> Result<()> {
    if !file.exists() {
        anyhow::bail!("File does not exist: {}", file.display());
    }

    let file_str = file.display().to_string();

    // --- Forward dependencies: what does this file import? ---
    let (grep, source) = parse_file(file)?;
    let root = grep.root();
    let imports = extract_imports(&root, &source);

    println!("Dependencies for: {file_str}");
    println!();

    if imports.is_empty() {
        println!("Imports: (none)");
    } else {
        println!("Imports:");
        for imp in &imports {
            println!(
                "  {}: {} | {}",
                imp.line + 1,
                imp.source,
                imp.line_text.trim()
            );
        }
    }

    // --- Reverse dependencies: who imports this file? ---
    // Walk from project root to find all files that import this one
    let search_dir = find_project_root(file);
    let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");

    let mut imported_by: Vec<(String, usize, String)> = Vec::new();

    let walker = ignore::WalkBuilder::new(&search_dir)
        .hidden(true)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_file() || entry_path == file {
            continue;
        }
        if detect_lang(entry_path).is_err() {
            continue;
        }
        if let Ok((grep2, source2)) = parse_file(entry_path) {
            let root2 = grep2.root();
            let their_imports = extract_imports(&root2, &source2);
            for imp in their_imports {
                if import_matches_file(&imp.source, file_stem, file_name) {
                    imported_by.push((entry_path.display().to_string(), imp.line, imp.line_text));
                }
            }
        }
    }

    println!();
    if imported_by.is_empty() {
        println!("Imported by: (none found in {})", search_dir.display());
    } else {
        println!("Imported by:");
        for (f, line, text) in &imported_by {
            println!("  {}:{} | {}", f, line + 1, text.trim());
        }
    }

    Ok(())
}
