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
            all,
        } => commands::jump::run(symbol, path.as_deref(), all),
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
        Command::Tree { ref path, depth } => commands::tree::run(path.as_deref(), depth),
        Command::Sg { ref args } => commands::sg::run(args),
        Command::Callers {
            ref symbol,
            ref path,
        } => commands::callers::run(symbol, path.as_deref()),
        Command::Deps { ref file } => commands::deps::run(file),
        Command::Diff {
            ref symbol,
            ref path,
        } => commands::diff::run(symbol, path.as_deref()),
        Command::Outline { ref path } => commands::outline::run(path.as_deref()),
        Command::Types {
            ref symbol,
            ref path,
            depth,
        } => commands::types::run(symbol, path.as_deref(), depth),
        Command::Scope { ref file, line } => commands::scope::run(file, line),
        Command::External(args) => {
            // Fallback: forward unknown commands to system shell
            let (cmd, cmd_args) = args.split_first().expect("external subcommand requires a command name");
            let status = process::Command::new(cmd)
                .args(cmd_args)
                .status();
            match status {
                Ok(s) => process::exit(s.code().unwrap_or(1)),
                Err(e) => {
                    eprintln!("navi: command not found: {cmd} ({e})");
                    process::exit(127);
                }
            }
        }
    };

    match result {
        Ok(()) => process::exit(0),
        Err(e) => {
            let err_str = format!("{e:#}");
            eprintln!("Error: {e}");

            // Exit 1: file/path errors
            if err_str.contains("Cannot read file")
                || err_str.contains("does not exist")
                || err_str.contains("Unsupported file")
                || err_str.contains("Path does not exist")
            {
                process::exit(1);
            }
            // Exit 2: argument/usage errors
            if err_str.contains("Invalid range")
                || err_str.contains("Invalid start")
                || err_str.contains("must be")
                || err_str.contains("beyond end of file")
            {
                process::exit(2);
            }
            // Exit 3: internal/AST errors
            process::exit(3);
        }
    }
}
