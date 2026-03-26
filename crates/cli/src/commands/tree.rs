use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};
use crate::formatter;

pub fn run(path: Option<&Path>, max_depth: Option<usize>, min_files: Option<usize>, all: bool) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    if search_dir.is_file() {
        return print_file_skeleton(search_dir);
    }

    // --all mode: show full directory structure like unix `tree`
    if all {
        return walk_and_print_all(search_dir, max_depth);
    }

    // If --n is specified, we do an adaptive walk: start with depth 1 and keep
    // increasing until we reach the requested minimum number of files.
    if let Some(min) = min_files {
        let mut depth = 2;
        loop {
            let count = count_code_files(search_dir, depth)?;
            if count >= min || depth > 20 {
                return walk_and_print(search_dir, Some(depth));
            }
            depth += 1;
        }
    }

    walk_and_print(search_dir, max_depth)
}

/// Count how many code files (parseable by ast-grep) exist within `dir` up to `max_depth`.
fn count_code_files(dir: &Path, max_depth: usize) -> Result<usize> {
    let mut builder = ignore::WalkBuilder::new(dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .max_depth(Some(max_depth));
    let walker = builder.build();

    let mut count = 0;
    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_file() && detect_lang(entry_path).is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

/// Walk directory and print skeleton for each code file.
/// Uses compact mode for directories: "File: path (N symbols)" with summary,
/// expanding full skeleton only when few files are present.
fn walk_and_print(dir: &Path, max_depth: Option<usize>) -> Result<()> {
    let mut builder = ignore::WalkBuilder::new(dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .sort_by_file_path(|a, b| a.cmp(b));
    if let Some(d) = max_depth {
        builder.max_depth(Some(d));
    }
    let walker = builder.build();

    // First pass: collect all code files
    let mut code_files: Vec<std::path::PathBuf> = Vec::new();
    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        if detect_lang(entry_path).is_err() {
            continue;
        }
        code_files.push(entry_path.to_path_buf());
    }

    // Decide mode: compact for >20 files, full skeleton for <=20
    let compact = code_files.len() > 20;
    let mut total_symbols = 0usize;

    for file_path in &code_files {
        if compact {
            print_file_compact(file_path, &mut total_symbols)?;
        } else {
            let _ = print_file_skeleton(file_path);
        }
    }

    if compact {
        println!();
        println!(
            "({} files, {} symbols total)",
            code_files.len(),
            total_symbols
        );
    }

    Ok(())
}

/// Walk directory and print ALL files (not just code files) in a tree structure.
/// Code files get symbol counts; other files shown by name only.
fn walk_and_print_all(dir: &Path, max_depth: Option<usize>) -> Result<()> {
    let mut builder = ignore::WalkBuilder::new(dir);
    builder
        .hidden(true)
        .git_ignore(true)
        .sort_by_file_path(|a, b| a.cmp(b));
    if let Some(d) = max_depth {
        builder.max_depth(Some(d));
    }
    let walker = builder.build();

    let dir_prefix = dir.to_string_lossy().to_string();
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut code_file_count = 0usize;
    let mut total_symbols = 0usize;

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();

        // Compute relative path for clean display
        let display_path = entry_path.display().to_string();
        let rel = if display_path.starts_with(&dir_prefix) {
            let stripped = &display_path[dir_prefix.len()..];
            stripped.trim_start_matches('/')
        } else {
            &display_path
        };

        // Skip the root directory itself
        if rel.is_empty() {
            continue;
        }

        // Calculate indent based on path depth
        let depth = rel.matches('/').count();
        let indent = "  ".repeat(depth);

        if entry_path.is_dir() {
            dir_count += 1;
            println!("{}{}/", indent, entry_path.file_name().unwrap_or_default().to_string_lossy());
        } else {
            file_count += 1;
            let fname = entry_path.file_name().unwrap_or_default().to_string_lossy().to_string();

            if detect_lang(entry_path).is_ok() {
                // Code file — show symbol count
                code_file_count += 1;
                match parse_file(entry_path) {
                    Ok((grep, _source)) => {
                        let root = grep.root();
                        let defs = collect_definitions(&root);
                        let top_level = defs.iter().filter(|d| d.depth == 0).count();
                        total_symbols += top_level;
                        if top_level > 0 {
                            println!("{}{}  ({} symbols)", indent, fname, top_level);
                        } else {
                            println!("{}{}", indent, fname);
                        }
                    }
                    Err(_) => {
                        println!("{}{}", indent, fname);
                    }
                }
            } else {
                // Non-code file — just show name
                println!("{}{}", indent, fname);
            }
        }
    }

    println!();
    println!(
        "({} directories, {} files, {} code files, {} symbols)",
        dir_count, file_count, code_file_count, total_symbols
    );
    Ok(())
}

/// Print compact summary: "File: path (N symbols)" — always shows every file.
fn print_file_compact(file: &Path, total: &mut usize) -> Result<()> {
    let file_str = file.display().to_string();
    match parse_file(file) {
        Ok((grep, _source)) => {
            let root = grep.root();
            let defs = collect_definitions(&root);
            let top_level = defs.iter().filter(|d| d.depth == 0).count();
            *total += top_level;
            println!("  {} ({} symbols)", file_str, top_level);
        }
        Err(_) => {
            println!("  {} (parse error)", file_str);
        }
    }
    Ok(())
}

fn print_file_skeleton(file: &Path) -> Result<()> {
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);

    let file_str = file.display().to_string();

    if defs.is_empty() {
        println!("File: {file_str}  (0 symbols)");
        return Ok(());
    }

    println!("File: {file_str}");
    for def in &defs {
        let skeleton = formatter::format_skeleton_line(&def.text, def.start_line, def.depth);
        println!("{skeleton}");
    }

    Ok(())
}
