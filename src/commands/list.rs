use anyhow::Result;
use std::path::Path;

use crate::ast::engine::{collect_definitions, parse_file};
use crate::formatter;

pub fn run(file: &Path) -> Result<()> {
    let (grep, _source) = parse_file(file)?;
    let root = grep.root();
    let defs = collect_definitions(&root);

    let file_str = file.display().to_string();
    println!("File: {}", file_str);

    if defs.is_empty() {
        println!("No results found for '{}'", file_str);
        return Ok(());
    }

    for def in &defs {
        let skeleton = formatter::format_skeleton_line(&def.text, def.start_line);
        println!("{}", skeleton);
    }

    Ok(())
}
