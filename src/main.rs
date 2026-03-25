mod ast;
mod cli;
mod commands;
mod formatter;

use clap::Parser;
use cli::{Command, NaviCli};
use std::process;

fn main() {
    let cli = NaviCli::parse();

    let result = match cli.command {
        Command::List { ref file } => {
            if !file.exists() {
                eprintln!("Error: File does not exist: {}", file.display());
                process::exit(1);
            }
            commands::list::run(file)
        }
        Command::Jump {
            ref symbol,
            ref path,
        } => commands::jump::run(symbol, path.as_deref()),
        Command::Refs {
            ref symbol,
            ref path,
        } => commands::refs::run(symbol, path.as_deref()),
        Command::Read {
            ref file,
            ref range,
        } => {
            if !file.exists() {
                eprintln!("Error: File does not exist: {}", file.display());
                process::exit(1);
            }
            commands::read::run(file, range)
        }
        Command::Init { ref path } => commands::init::run(path.as_deref()),
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            let err_str = format!("{:#}", e);
            if err_str.contains("Cannot read file") || err_str.contains("does not exist") {
                eprintln!("Error: {}", e);
                process::exit(1);
            } else if err_str.contains("Invalid range") || err_str.contains("Invalid start") {
                eprintln!("Error: {}", e);
                process::exit(2);
            } else {
                eprintln!("Error: {}", e);
                process::exit(3);
            }
        }
    }
}
