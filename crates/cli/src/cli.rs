use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "navi",
    about = "Headless code navigation CLI for AI agents",
    allow_external_subcommands = true
)]
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
        /// Show all definitions instead of just the first
        #[arg(long)]
        all: bool,
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
        /// Show inline type hints (IDE-style annotations)
        #[arg(long)]
        hints: bool,
    },
    /// Initialize Navi skill document in .agent/skills/navi/
    Init {
        /// Optional base directory (defaults to CWD)
        path: Option<PathBuf>,
    },
    /// Recursively list skeleton of all files in a directory
    Tree {
        /// Optional directory to scan (defaults to CWD)
        path: Option<PathBuf>,
        /// Max directory depth to recurse into
        #[arg(long)]
        depth: Option<usize>,
        /// Minimum number of files to display (auto-adjusts depth)
        #[arg(short, long)]
        n: Option<usize>,
    },
    /// Passthrough to ast-grep CLI (run, scan, test, etc.)
    #[command(trailing_var_arg = true)]
    Sg {
        /// Arguments forwarded to ast-grep
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Find call-site usages of a symbol (excludes imports, type annotations)
    Callers {
        /// Symbol name to search for
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Show file dependencies (imports and reverse imports)
    Deps {
        /// Path to the source file
        file: PathBuf,
    },
    /// Show git diff filtered to a specific symbol
    Diff {
        /// Symbol name to filter diff for
        symbol: Option<String>,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Show symbols changed in the last N commits (summary mode)
        #[arg(long)]
        since: Option<usize>,
    },
    /// Find all implementations of a trait/interface
    Impls {
        /// Trait or interface name to search for
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Show project-level architecture overview with package dependencies
    Outline {
        /// Optional directory to scan (defaults to CWD)
        path: Option<PathBuf>,
    },
    /// Recursively expand a type and its referenced types
    Types {
        /// Symbol name to look up
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Max depth of recursive type expansion (default: 1)
        #[arg(long, default_value = "1")]
        depth: usize,
    },
    /// Show the enclosing scope (function/method) for a given file and line
    Scope {
        /// Path to the source file
        file: PathBuf,
        /// Line number (1-indexed)
        line: usize,
    },
    /// AST-aware grep: search for an identifier and show enclosing function context
    Grep {
        /// Identifier pattern to search for
        pattern: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// List exported/public symbols from a file or directory
    Exports {
        /// Path to file or directory
        path: PathBuf,
    },
    /// Trace the caller chain of a function up to N levels
    Flow {
        /// Entry function name
        symbol: String,
        /// Optional directory to search in (defaults to CWD)
        #[arg(long)]
        path: Option<PathBuf>,
        /// Max depth of caller chain expansion (default: 2)
        #[arg(long, default_value = "2")]
        depth: usize,
    },
    /// Fallback: forward unknown commands to system shell
    #[command(external_subcommand)]
    External(Vec<String>),
}
