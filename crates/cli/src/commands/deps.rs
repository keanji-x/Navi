use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{detect_lang, extract_imports, parse_file};

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
    // Walk from the file's parent directory (or CWD) to find files that import this one
    let search_dir = file.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_name = file.file_name().and_then(|s| s.to_str()).unwrap_or("");

    let mut imported_by: Vec<(String, usize, String)> = Vec::new();

    let walker = ignore::WalkBuilder::new(search_dir)
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
                // Check if the import source references our file
                // Match by file stem, file name, or path suffix
                if imp.source.contains(file_stem)
                    || imp.source.contains(file_name)
                    || imp.source.ends_with(file_stem)
                {
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
