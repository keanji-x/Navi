use anyhow::Result;
use regex::Regex;
use std::path::Path;

use crate::ast::engine::{collect_definitions, detect_lang, parse_file};

pub fn run(pattern: &str, path: Option<&Path>, kind_filter: Option<&str>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    let re = Regex::new(pattern)
        .map_err(|e| anyhow::anyhow!("Invalid regex pattern '{}': {}", pattern, e))?;

    let mut results: Vec<(String, usize, String, String)> = Vec::new(); // (file, line, kind, name)

    let walker = ignore::WalkBuilder::new(search_dir)
        .hidden(true)
        .git_ignore(true)
        .sort_by_file_path(|a, b| a.cmp(b))
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
        if let Ok((grep, _source)) = parse_file(entry_path) {
            let root = grep.root();
            let defs = collect_definitions(&root);
            let file_str = entry_path.display().to_string();

            for def in &defs {
                if let Some(ref name) = def.name {
                    if !re.is_match(name) {
                        continue;
                    }
                    let kind_label = normalize_kind(&def.kind);
                    if let Some(filter) = kind_filter {
                        if kind_label != filter {
                            continue;
                        }
                    }
                    results.push((
                        file_str.clone(),
                        def.start_line,
                        kind_label.to_string(),
                        name.clone(),
                    ));
                }
            }
        }
    }

    if results.is_empty() {
        let filter_msg = kind_filter
            .map(|k| format!(" (kind: {k})"))
            .unwrap_or_default();
        println!("No symbols matching '{pattern}'{filter_msg}");
    } else {
        println!("Found {} symbols matching '{pattern}':", results.len());
        for (file, line, kind, name) in &results {
            println!("  {}:{} {} {}", file, line + 1, kind, name);
        }
    }
    Ok(())
}

/// Normalize tree-sitter node kind to a user-friendly label.
fn normalize_kind(kind: &str) -> &str {
    match kind {
        "function_declaration" | "function_definition" | "function_item"
        | "arrow_function" | "generator_function_declaration" => "function",
        "method_definition" | "method_declaration" | "function_signature_item" => "method",
        "class_declaration" | "class_definition" => "class",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "interface_declaration" => "interface",
        "type_alias_declaration" | "type_item" => "type",
        "trait_item" => "trait",
        "impl_item" => "impl",
        "lexical_declaration" | "variable_declaration" | "const_item" | "static_item" => "const",
        "mod_item" => "mod",
        "export_statement" => "export",
        _ => kind,
    }
}
