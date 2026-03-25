use anyhow::{bail, Context, Result};
use std::path::Path;

use crate::formatter;

pub fn run(file: &Path, range: &str) -> Result<()> {
    if !file.exists() {
        bail!("File does not exist: {}", file.display());
    }

    let source = std::fs::read_to_string(file)
        .with_context(|| format!("Cannot read file: {}", file.display()))?;

    let (start, end) = parse_range(range)?;

    let output = formatter::format_read_output(&file.display().to_string(), &source, start, end);
    print!("{output}");
    Ok(())
}

/// Parse a range string like "10-20" into (start, end) as 1-indexed.
fn parse_range(range: &str) -> Result<(usize, usize)> {
    let parts: Vec<&str> = range.split('-').collect();
    if parts.len() != 2 {
        bail!("Invalid range format '{range}'. Expected START-END (e.g., 10-20)");
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
