use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::ast::engine::{collect_definitions, parse_file};
use crate::formatter;

pub fn run(file: &Path, range: &str) -> Result<()> {
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

        let def = defs.iter().find(|d| d.name.as_deref() == Some(range));

        match def {
            Some(d) => {
                let output = formatter::format_read_output(
                    &file_str,
                    &source,
                    d.start_line + 1, // convert 0-based to 1-based
                    d.end_line + 1,
                );
                print!("{output}");
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
        let output = formatter::format_read_output(&file_str, &source, start, end);
        print!("{output}");
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
