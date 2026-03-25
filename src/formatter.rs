/// Format a definition skeleton line for `navi list`.
/// Replaces the body (everything after the first `{` or `:`) with `{ ... }`.
pub fn format_skeleton_line(text: &str, start_line: usize) -> String {
    let first_line = text.lines().next().unwrap_or(text);

    // Try to find the opening brace to truncate the body
    if let Some(brace_pos) = first_line.find('{') {
        let signature = first_line[..brace_pos].trim_end();
        format!("{:>4}: {} {{ ... }}", start_line + 1, signature)
    } else if let Some(colon_pos) = first_line.find(':') {
        // Python-style: `def foo():` or `class Foo:`
        let signature = &first_line[..=colon_pos];
        format!("{:>4}: {} ...", start_line + 1, signature.trim_end())
    } else {
        // Fallback: just show the first line
        format!("{:>4}: {}", start_line + 1, first_line.trim_end())
    }
}

/// Format the complete output for `navi list`.
#[allow(dead_code)]
pub fn format_list_output(file_path: &str, skeletons: &[(usize, String)]) -> String {
    let mut out = format!("File: {}\n", file_path);
    for (line, text) in skeletons {
        out.push_str(&format!("{:>4}: {}\n", line + 1, text));
    }
    out
}

/// Format a definition with context lines for `navi jump`.
pub fn format_jump_output(
    file_path: &str,
    symbol: &str,
    source: &str,
    start_line: usize,
    end_line: usize,
    context: usize,
) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let ctx_start = start_line.saturating_sub(context);
    let ctx_end = (end_line + context).min(lines.len().saturating_sub(1));

    let mut out = format!("Found definition for '{}' in {}:\n", symbol, file_path);
    for i in ctx_start..=ctx_end {
        if let Some(line) = lines.get(i) {
            out.push_str(&format!("{:>4}: {}\n", i + 1, line));
        }
    }
    out
}

/// Format references output for `navi refs`.
pub fn format_refs_output(symbol: &str, refs: &[(String, usize, String)]) -> String {
    if refs.is_empty() {
        return format!("No results found for '{}'\n", symbol);
    }
    let mut out = format!("Found {} references for '{}':\n", refs.len(), symbol);
    for (file, line, text) in refs {
        out.push_str(&format!("- {}: {} | {}\n", file, line + 1, text.trim()));
    }
    out
}

/// Format line-range read output for `navi read`.
pub fn format_read_output(file_path: &str, source: &str, start: usize, end: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::new();
    // start and end are 1-indexed from CLI, convert to 0-indexed
    let start_idx = start.saturating_sub(1);
    let end_idx = end.min(lines.len());

    for i in start_idx..end_idx {
        if let Some(line) = lines.get(i) {
            out.push_str(&format!("{:>4}: {}\n", i + 1, line));
        }
    }
    if out.is_empty() {
        out = format!(
            "No results found for '{}' lines {}-{}\n",
            file_path, start, end
        );
    }
    out
}
