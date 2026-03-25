use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "navi", about = "Headless code navigation CLI for AI agents")]
pub struct NaviCli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Extract file skeleton (classes, functions, interfaces with bodies collapsed)
    List {
        /// Path to the source file
        file: PathBuf,
    },
    /// Jump to the full definition of a symbol
    Jump {
        /// Symbol name to look up
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Find all references to a symbol
    Refs {
        /// Symbol name to search for
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Read a specific line range from a file
    Read {
        /// Path to the source file
        file: PathBuf,
        /// Line range in START-END format (1-indexed)
        range: String,
    },
    /// Initialize Navi skill document in .agent/skills/navi/
    Init {
        /// Optional base directory (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
}
