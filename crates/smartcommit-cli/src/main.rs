//! SmartCommit CLI binary.
//!
//! # Module 1 status
//! Adds the `doctor` subcommand, which reports repository detection
//! only (state inspection — branch, conflicts, staged/unstaged
//! status — is Module 2's scope, not this one).

use clap::{Parser, Subcommand};
use smartcommit_core::{Repository, RepositoryError};
use std::path::PathBuf;
use std::process::ExitCode;

/// SmartCommit — a professional Git commit-planning and splitting
/// tool.
#[derive(Parser, Debug)]
#[command(name = "smartcommit", version, about, long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    /// Path to operate on. Defaults to the current directory.
    /// Accepted anywhere along the repo-discovery path (repo root,
    /// or a directory nested inside it).
    #[arg(long, global = true, value_name = "PATH")]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check whether the target path is a Git repository.
    ///
    /// Module 1 scope: repository detection only. Branch, conflict,
    /// and staged/unstaged status reporting arrive in Module 2.
    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => run_doctor(cli.repo),
    }
}

fn run_doctor(repo_path: Option<PathBuf>) -> ExitCode {
    // Default to the current directory when --repo isn't given.
    // current_dir() failing (e.g. the cwd was deleted out from
    // under the process) is an environment problem, not a user
    // input problem — report it plainly rather than folding it into
    // RepositoryError's cases, which are all about the *target*
    // path being invalid.
    let target = match repo_path {
        Some(p) => p,
        None => match std::env::current_dir() {
            Ok(cwd) => cwd,
            Err(e) => {
                eprintln!("Could not determine the current directory: {e}");
                return ExitCode::FAILURE;
            }
        },
    };

    match Repository::discover(&target) {
        Ok(info) => {
            println!("Repository detected");
            println!("Root: {}", info.root.display());
            if info.is_bare {
                println!("Bare repository: yes");
            }
            ExitCode::SUCCESS
        }
        Err(RepositoryError::NotARepository(_)) => {
            println!("No Git repository found.");
            ExitCode::FAILURE
        }
        Err(RepositoryError::PathNotFound(path)) => {
            eprintln!("Path does not exist: {}", path.display());
            ExitCode::FAILURE
        }
        Err(RepositoryError::PathNotAccessible { path, reason }) => {
            eprintln!("Path is not accessible: {} ({reason})", path.display());
            ExitCode::FAILURE
        }
        Err(RepositoryError::GitUnavailable(msg)) => {
            eprintln!("Unable to run 'git': {msg}");
            eprintln!();
            eprintln!("Possible causes:");
            eprintln!("- git is not installed or not on PATH");
            ExitCode::FAILURE
        }
    }
}

