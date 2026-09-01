//! SmartCommit CLI binary.
//!
//! # Module 0 status
//! Only `--version` and `--help` are implemented. Subcommands
//! (`doctor`, `analyze`, `plan`, `commit`, `restore`, ...) are added
//! in later modules as the corresponding core functionality lands.

use clap::Parser;

/// SmartCommit — a professional Git commit-planning and splitting
/// tool.
#[derive(Parser, Debug)]
#[command(name = "smartcommit", version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {}

fn main() {
    // Module 0: parsing is the entire behavior. clap handles
    // --version and --help (and shows help when no args/flags are
    // given, via arg_required_else_help) without any code here.
    // Subcommand dispatch arrives in Module 1+.
    let _cli = Cli::parse();
}
