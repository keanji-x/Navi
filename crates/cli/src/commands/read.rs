use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::ast::engine::{collect_definitions, parse_file};
use crate::formatter;

pub fn run(file: &Path, range: &str, hints: bool) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let source = std::fs::read_to_string(file)
        .with_context(|| format!("Cannot read file: {}", file.display()))?;

    let file_str = file.display().to_string();

    // Check if range is a symbol name (non-numeric, no separator)
    if is_symbol_name(range) {
        let (grep, _source) = parse_file(file)?;
        let root = grep.root();
        let defs = collect_definitions(&root);

        // Support dot-path: "Class.method" or "Struct.field"
        let def = if range.contains('.') {
            let parts: Vec<&str> = range.splitn(2, '.').collect();
            let parent_name = parts[0];
            let child_name = parts[1];
            // Find parent definition
            let parent = defs.iter().find(|d| d.name.as_deref() == Some(parent_name));
            match parent {
                Some(p) => {
                    // Find child within parent's line range
                    defs.iter().find(|d| {
                        d.name.as_deref() == Some(child_name)
                            && d.start_line >= p.start_line
                            && d.end_line <= p.end_line
                            && d.depth > p.depth
                    })
                }
                None => None,
            }
        } else {
            defs.iter().find(|d| d.name.as_deref() == Some(range))
        };

        match def {
            Some(d) => {
                let start = d.start_line + 1; // convert 0-based to 1-based
                let end = d.end_line + 1;
                if hints {
                    let hint_map = extract_type_hints(&root, d.start_line, d.end_line);
                    let output = format_read_with_hints(&file_str, &source, start, end, &hint_map);
                    print!("{output}");
                } else {
                    let output = formatter::format_read_output(&file_str, &source, start, end);
                    print!("{output}");
                }
            }
            None => {
                // Try fuzzy match for helpful error
                let names: Vec<&str> = defs
                    .iter()
                    .filter_map(|d| d.name.as_deref())
                    .collect();
                let mut msg = format!("Symbol '{}' not found in {}", range, file_str);
                let similar: Vec<&&str> = names
                    .iter()
                    .filter(|n| n.contains(range) || range.contains(**n))
                    .collect();
                if !similar.is_empty() {
                    msg.push_str(". Similar symbols:");
                    for s in similar.iter().take(5) {
                        msg.push_str(&format!(" {s}"));
                    }
                }
                bail!("{msg}");
            }
        }
    } else {
        let (start, end) = parse_range(range)?;
        if hints {
            if let Ok((grep, _)) = parse_file(file) {
                let root = grep.root();
                let hint_map =
                    extract_type_hints(&root, start.saturating_sub(1), end.saturating_sub(1));
                let output = format_read_with_hints(&file_str, &source, start, end, &hint_map);
                print!("{output}");
            } else {
                // Fallback: can't parse AST, just show plain output
                let output = formatter::format_read_output(&file_str, &source, start, end);
                print!("{output}");
            }
        } else {
            let output = formatter::format_read_output(&file_str, &source, start, end);
            print!("{output}");
        }
    }

    Ok(())
}

/// Check if the range string is a symbol name rather than a line range.
/// A symbol name contains no digits-only segments separated by '-' or ':'.
fn is_symbol_name(range: &str) -> bool {
    // If it contains '::' prefix (e.g. "::my_func"), strip it
    let r = range.strip_prefix("::").unwrap_or(range);

    // A numeric range has format: digits separator digits (e.g. "10-20", "10:20")
    // If the string starts with a digit and contains '-' or ':', treat as range
    if r.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return false;
    }

    // Otherwise it's a symbol name
    true
}

/// Parse a range string like "10-20" or "10:20" into (start, end) as 1-indexed.
fn parse_range(range: &str) -> Result<(usize, usize)> {
    // Support both '-' and ':' as separators (e.g. "10-20" or "10:20")
    let sep = if range.contains('-') { '-' } else { ':' };
    let parts: Vec<&str> = range.splitn(2, sep).collect();
    if parts.len() != 2 {
        bail!("Invalid range format '{range}'. Expected START-END (e.g., 10-20 or 10:20)");
    }

    let start: usize = parts[0]
        .parse()
        .with_context(|| format!("Invalid start line: '{}'", parts[0]))?;
    let end: usize = parts[1]
        .parse()
        .with_context(|| format!("Invalid end line: '{}'", parts[1]))?;

    if start == 0 {
        bail!("Start line must be >= 1");
    }
    if end < start {
        bail!("End line ({end}) must be >= start line ({start})");
    }

    Ok((start, end))
}

/// A type hint extracted from the AST, to be displayed inline.
struct TypeHint {
    /// The name being annotated (variable, parameter, field, etc.)
    name: String,
    /// The type string
    type_text: String,
}

/// Extract type hints from AST nodes within a 0-based line range.
/// Returns a map from 0-based line number to list of hints on that line.
fn extract_type_hints(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    start_line: usize,
    end_line: usize,
) -> HashMap<usize, Vec<TypeHint>> {
    let mut hints: HashMap<usize, Vec<TypeHint>> = HashMap::new();

    for node in root.dfs() {
        let line = node.start_pos().line();
        if line < start_line || line > end_line {
            continue;
        }

        let kind = node.kind();
        let kind_str = kind.as_ref();

        match kind_str {
            // --- Function / method return type ---
            // e.g. fn foo() -> Result<()>  or  function bar(): string
            "function_item" | "function_declaration" | "function_definition"
            | "method_definition" | "method_declaration" | "function_signature_item"
            | "arrow_function" => {
                extract_function_hints(&node, &mut hints);
            }

            // --- Let bindings with type annotation ---
            // e.g. let x: Vec<String> = ...
            "let_declaration" => {
                extract_let_hints(&node, &mut hints);
            }

            // --- Parameter with type annotation ---
            // e.g. fn foo(x: i32)  or  function bar(x: number)
            "parameter" | "required_parameter" | "optional_parameter" | "typed_parameter" => {
                extract_param_hints(&node, &mut hints);
            }

            // --- Field declarations with type ---
            // e.g. pub name: String
            "field_declaration" | "property_signature" | "property_declaration"
            | "public_field_definition" => {
                extract_field_hints(&node, &mut hints);
            }

            _ => {}
        }
    }

    hints
}

/// Extract return type hint from a function node.
fn extract_function_hints(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    hints: &mut HashMap<usize, Vec<TypeHint>>,
) {
    let fn_name = node
        .field("name")
        .map(|n| n.text().to_string())
        .unwrap_or_default();
    if fn_name.is_empty() {
        return;
    }

    // Look for return_type field (Rust: `-> Type`, TS: `: Type`)
    if let Some(ret) = node.field("return_type") {
        let ret_text = ret.text().to_string();
        // Clean up: remove leading `->` or `:` if present in the text
        let cleaned = ret_text
            .trim_start_matches("->")
            .trim_start_matches(':')
            .trim()
            .to_string();
        if !cleaned.is_empty() {
            let line = node.start_pos().line();
            hints.entry(line).or_default().push(TypeHint {
                name: fn_name,
                type_text: format!("→ {cleaned}"),
            });
        }
    }
}

/// Extract type hint from a let declaration.
fn extract_let_hints(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    hints: &mut HashMap<usize, Vec<TypeHint>>,
) {
    // Try to find `pattern` and `type` fields
    let var_name = node
        .field("pattern")
        .map(|n| n.text().to_string())
        .unwrap_or_default();

    if let Some(ty) = node.field("type") {
        let type_text = ty.text().to_string();
        let cleaned = type_text.trim_start_matches(':').trim().to_string();
        if !cleaned.is_empty() && !var_name.is_empty() {
            let line = node.start_pos().line();
            hints.entry(line).or_default().push(TypeHint {
                name: var_name,
                type_text: format!(": {cleaned}"),
            });
        }
    }
}

/// Extract type hint from a parameter node.
fn extract_param_hints(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    hints: &mut HashMap<usize, Vec<TypeHint>>,
) {
    let param_name = node
        .field("pattern")
        .or_else(|| node.field("name"))
        .map(|n| n.text().to_string())
        .unwrap_or_default();

    if let Some(ty) = node.field("type") {
        let type_text = ty.text().to_string();
        let cleaned = type_text.trim_start_matches(':').trim().to_string();
        if !cleaned.is_empty() && !param_name.is_empty() {
            let line = node.start_pos().line();
            hints.entry(line).or_default().push(TypeHint {
                name: param_name,
                type_text: format!(": {cleaned}"),
            });
        }
    }
}

/// Extract type hint from a field declaration node.
fn extract_field_hints(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<ast_grep_language::SupportLang>>,
    hints: &mut HashMap<usize, Vec<TypeHint>>,
) {
    let field_name = node
        .field("name")
        .map(|n| n.text().to_string())
        .unwrap_or_default();

    if let Some(ty) = node.field("type") {
        let type_text = ty.text().to_string();
        let cleaned = type_text.trim_start_matches(':').trim().to_string();
        if !cleaned.is_empty() && !field_name.is_empty() {
            let line = node.start_pos().line();
            hints.entry(line).or_default().push(TypeHint {
                name: field_name,
                type_text: format!(": {cleaned}"),
            });
        }
    }
}

/// Format read output with inline type hints appended as comments.
fn format_read_with_hints(
    file_path: &str,
    source: &str,
    start: usize,
    end: usize,
    hint_map: &HashMap<usize, Vec<TypeHint>>,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    let start_idx = start.saturating_sub(1);
    let end_idx = end.min(lines.len());

    // Detect comment style from file extension
    let comment_prefix = if file_path.ends_with(".py") {
        "#"
    } else {
        "//"
    };

    for i in start_idx..end_idx {
        if let Some(line) = lines.get(i) {
            // Check if there are hints for this line (using 0-based index)
            if let Some(line_hints) = hint_map.get(&i) {
                let hint_str: Vec<String> = line_hints
                    .iter()
                    .map(|h| format!("{} {}", h.name, h.type_text))
                    .collect();
                let joined = hint_str.join(", ");
                out.push_str(&format!(
                    "{:>4}: {}  {} hint: {}\n",
                    i + 1,
                    line,
                    comment_prefix,
                    joined
                ));
            } else {
                out.push_str(&format!("{:>4}: {}\n", i + 1, line));
            }
        }
    }
    if out.is_empty() {
        out = format!("No results found for '{file_path}' lines {start}-{end}\n");
    }
    out
}
