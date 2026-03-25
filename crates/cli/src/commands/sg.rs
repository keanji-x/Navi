use anyhow::{Context, Result};
use std::process::Command;

pub fn run(args: &[String]) -> Result<()> {
    let status = Command::new("ast-grep").args(args).status().context(
        "Failed to execute ast-grep. Is it installed?\n\
             Install with: cargo install ast-grep",
    )?;

    std::process::exit(status.code().unwrap_or(1));
}
