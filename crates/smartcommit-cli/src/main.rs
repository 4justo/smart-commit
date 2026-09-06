//! SmartCommit CLI binary.
//!
//! # Module 3 status
//! Adds the `analyze` subcommand, which lists staged files only —
//! diff content (hunks, additions/deletions) is Module 4's scope.

use clap::{Parser, Subcommand};
use smartcommit_core::{
    inspect_repository_state, get_staged_changes, ChangeKind, InspectorError, RepositoryError,
    RepositorySafety, StagedChangesError,
};
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
    /// Check repository detection and full state: branch, HEAD,
    /// in-progress merge/rebase/cherry-pick, conflicts, and
    /// staged/unstaged/untracked status.
    Doctor,
    /// List currently staged files and their change kind.
    ///
    /// Module 3 scope: the staged file list only. Diff content
    /// (hunks, additions/deletions) arrives in Module 4.
    Analyze,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => run_doctor(cli.repo),
        Commands::Analyze => run_analyze(cli.repo),
    }
}

/// Resolves the target path: `--repo` if given, else the current
/// directory. Shared by every subcommand that operates on a
/// repository.
fn resolve_target(repo_path: Option<PathBuf>) -> Result<PathBuf, ExitCode> {
    match repo_path {
        Some(p) => Ok(p),
        None => std::env::current_dir().map_err(|e| {
            eprintln!("Could not determine the current directory: {e}");
            ExitCode::FAILURE
        }),
    }
}

fn run_doctor(repo_path: Option<PathBuf>) -> ExitCode {
    let target = match resolve_target(repo_path) {
        Ok(t) => t,
        Err(code) => return code,
    };

    match inspect_repository_state(&target) {
        Ok(state) => {
            println!("Repository: valid");
            println!(
                "Branch: {}",
                state.branch.as_deref().unwrap_or("(detached or unknown)")
            );
            println!("HEAD: {}", state.head.as_deref().unwrap_or("(unknown)"));
            println!();
            println!("Staged changes: {}", yes_no(state.has_staged_changes));
            println!("Unstaged changes: {}", yes_no(state.has_unstaged_changes));
            println!("Untracked files: {}", yes_no(state.has_untracked_files));
            println!();
            println!("State: {}", safety_label(state.safety()));
            ExitCode::SUCCESS
        }
        Err(InspectorError::Repository(RepositoryError::NotARepository(_))) => {
            println!("No Git repository found.");
            ExitCode::FAILURE
        }
        Err(InspectorError::Repository(RepositoryError::PathNotFound(path))) => {
            eprintln!("Path does not exist: {}", path.display());
            ExitCode::FAILURE
        }
        Err(InspectorError::Repository(RepositoryError::PathNotAccessible { path, reason })) => {
            eprintln!("Path is not accessible: {} ({reason})", path.display());
            ExitCode::FAILURE
        }
        Err(InspectorError::Repository(RepositoryError::GitUnavailable(msg)))
        | Err(InspectorError::GitUnavailable(msg)) => {
            eprintln!("Unable to run 'git': {msg}");
            eprintln!();
            eprintln!("Possible causes:");
            eprintln!("- git is not installed or not on PATH");
            ExitCode::FAILURE
        }
        Err(InspectorError::StatusFailed) => {
            eprintln!("Unable to read repository status.");
            eprintln!();
            eprintln!("git status did not complete successfully.");
            ExitCode::FAILURE
        }
    }
}

fn run_analyze(repo_path: Option<PathBuf>) -> ExitCode {
    let target = match resolve_target(repo_path) {
        Ok(t) => t,
        Err(code) => return code,
    };

    match get_staged_changes(&target) {
        Ok(files) if files.is_empty() => {
            println!("No staged changes found.");
            ExitCode::FAILURE
        }
        Ok(files) => {
            println!("Staged changes: {}", files.len());
            println!();
            for file in &files {
                let label = match file.change_kind {
                    ChangeKind::Added => "Added",
                    ChangeKind::Modified => "Modified",
                    ChangeKind::Deleted => "Deleted",
                    ChangeKind::Renamed => "Renamed",
                };
                match (&file.change_kind, &file.old_path) {
                    (ChangeKind::Renamed, Some(old)) => {
                        println!(
                            "{label:<9} {} -> {}",
                            old.display(),
                            file.path.display()
                        );
                    }
                    _ => {
                        println!("{label:<9} {}", file.path.display());
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(StagedChangesError::Repository(RepositoryError::NotARepository(_))) => {
            println!("No Git repository found.");
            ExitCode::FAILURE
        }
        Err(StagedChangesError::Repository(RepositoryError::PathNotFound(path))) => {
            eprintln!("Path does not exist: {}", path.display());
            ExitCode::FAILURE
        }
        Err(StagedChangesError::Repository(RepositoryError::PathNotAccessible {
            path,
            reason,
        })) => {
            eprintln!("Path is not accessible: {} ({reason})", path.display());
            ExitCode::FAILURE
        }
        Err(StagedChangesError::Repository(RepositoryError::GitUnavailable(msg)))
        | Err(StagedChangesError::GitUnavailable(msg)) => {
            eprintln!("Unable to run 'git': {msg}");
            eprintln!();
            eprintln!("Possible causes:");
            eprintln!("- git is not installed or not on PATH");
            ExitCode::FAILURE
        }
        Err(StagedChangesError::CommandFailed(msg)) => {
            eprintln!("Unable to read staged changes.");
            eprintln!();
            eprintln!("Details: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn safety_label(safety: RepositorySafety) -> &'static str {
    match safety {
        RepositorySafety::Safe => "ready",
        RepositorySafety::Warning => "warning",
        RepositorySafety::Blocked => "blocked",
    }
}



