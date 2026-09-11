//! SmartCommit CLI binary.
//!
//! # Module 4 status
//! `analyze` now shows a diff summary (+additions -deletions, hunk
//! count, or "Binary file changed") per staged file. `--verbose`
//! additionally prints full hunk content.
//!
//! # Module 7 status
//! `plan` now builds and prints a real `CommitPlan`: it reads the
//! messages file, reads staged files and diffs, scores every
//! message/file pair (Module 6), and groups files under the message
//! the planner is confident they belong to (Module 7). No Git
//! modification occurs — this command only reads.

use clap::{Parser, Subcommand};
use smartcommit_core::{
    build_commit_plan, get_staged_changes, get_staged_diffs, inspect_repository_state,
    match_files_to_messages, ChangeKind, CommitMessages, DiffLine, FileDiff, InspectorError,
    ParseCommitMessagesError, RepositoryError, RepositorySafety, StagedChangesError,
};
use std::fs;
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
    /// List currently staged files, their change kind, and a diff
    /// summary (or full hunk content with --verbose).
    Analyze {
        /// Print full hunk content (added/removed/context lines)
        /// for every non-binary file, not just the summary.
        #[arg(long)]
        verbose: bool,
    },
    /// Propose a commit plan from a messages file and the currently
    /// staged changes. Read-only — makes no Git modification.
    Plan {
        /// Path to a text file containing commit messages.
        #[arg(long, value_name = "FILE")]
        messages_file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor => run_doctor(cli.repo),
        Commands::Analyze { verbose } => run_analyze(cli.repo, verbose),
        Commands::Plan { messages_file } => run_plan(cli.repo, messages_file),
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

fn run_analyze(repo_path: Option<PathBuf>, verbose: bool) -> ExitCode {
    let target = match resolve_target(repo_path) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let files = match get_staged_changes(&target) {
        Ok(files) if files.is_empty() => {
            println!("No staged changes found.");
            return ExitCode::FAILURE;
        }
        Ok(files) => files,
        Err(e) => return report_staged_changes_error(e),
    };

    // A diff-extraction failure shouldn't hide the file list we
    // already have from get_staged_changes — fall back to listing
    // files without diff stats rather than losing the whole
    // command's output over a secondary failure.
    let diffs = match get_staged_diffs(&target) {
        Ok(diffs) => diffs,
        Err(e) => {
            eprintln!("Warning: could not extract diff content ({e}); showing file list only.");
            Vec::new()
        }
    };

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
                println!("{label:<9} {} -> {}", old.display(), file.path.display());
            }
            _ => {
                println!("{label:<9} {}", file.path.display());
            }
        }

        if let Some(diff) = diffs.iter().find(|d| d.path == file.path) {
            print_diff_summary(diff, verbose);
        }
        println!();
    }

    ExitCode::SUCCESS
}

fn run_plan(repo_path: Option<PathBuf>, messages_file: PathBuf) -> ExitCode {
    let text = match fs::read_to_string(&messages_file) {
        Ok(text) => text,
        Err(err) => {
            eprintln!(
                "Could not read messages file {}: {err}",
                messages_file.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let messages = match text.parse::<CommitMessages>() {
        Ok(messages) => messages,
        Err(ParseCommitMessagesError::EmptyInput) => {
            eprintln!(
                "Could not parse commit messages from {}: message input is empty",
                messages_file.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let target = match resolve_target(repo_path) {
        Ok(t) => t,
        Err(code) => return code,
    };

    let files = match get_staged_changes(&target) {
        Ok(files) if files.is_empty() => {
            println!("No staged changes found.");
            return ExitCode::FAILURE;
        }
        Ok(files) => files,
        Err(e) => return report_staged_changes_error(e),
    };

    // Same fallback approach as `analyze`: a diff-extraction failure
    // shouldn't block planning — diff content only affects the
    // matcher's diff-keyword evidence, not filename/directory
    // evidence, so a plan without it is still useful.
    let diffs = match get_staged_diffs(&target) {
        Ok(diffs) => diffs,
        Err(e) => {
            eprintln!("Warning: could not extract diff content ({e}); matching on filenames and directories only.");
            Vec::new()
        }
    };

    let scores = match_files_to_messages(&messages.0, &files, &diffs);
    let plan = build_commit_plan(&messages.0, &files, &scores);

    println!("Commit Plan");
    println!();
    for (i, group) in plan.groups.iter().enumerate() {
        println!("[{}] {}", i + 1, group.message);
        if group.files.is_empty() {
            println!("    (no matching files)");
        } else {
            for file in &group.files {
                println!("    {}", file.path.display());
            }
        }
        println!("    Confidence: {:.0}%", group.confidence * 100.0);
        println!();
    }

    if !plan.unassigned.is_empty() {
        println!("Unassigned:");
        for file in &plan.unassigned {
            println!("    {}", file.path.display());
        }
        println!();
    }

    if !plan.warnings.is_empty() {
        println!("Warnings:");
        for warning in &plan.warnings {
            println!("    {warning}");
        }
    }

    ExitCode::SUCCESS
}

fn print_diff_summary(diff: &FileDiff, verbose: bool) {
    if diff.is_binary {
        println!("  Binary file changed");
        return;
    }

    println!("  +{} -{}", diff.additions, diff.deletions);
    println!(
        "  {} hunk{}",
        diff.hunks.len(),
        if diff.hunks.len() == 1 { "" } else { "s" }
    );

    if verbose {
        for hunk in &diff.hunks {
            println!("  {}", hunk.header);
            for line in &hunk.lines {
                match line {
                    DiffLine::Added(text) => println!("  +{text}"),
                    DiffLine::Removed(text) => println!("  -{text}"),
                    DiffLine::Context(text) => println!("   {text}"),
                }
            }
        }
    }
}

fn report_staged_changes_error(err: StagedChangesError) -> ExitCode {
    match err {
        StagedChangesError::Repository(RepositoryError::NotARepository(_)) => {
            println!("No Git repository found.");
            ExitCode::FAILURE
        }
        StagedChangesError::Repository(RepositoryError::PathNotFound(path)) => {
            eprintln!("Path does not exist: {}", path.display());
            ExitCode::FAILURE
        }
        StagedChangesError::Repository(RepositoryError::PathNotAccessible { path, reason }) => {
            eprintln!("Path is not accessible: {} ({reason})", path.display());
            ExitCode::FAILURE
        }
        StagedChangesError::Repository(RepositoryError::GitUnavailable(msg))
        | StagedChangesError::GitUnavailable(msg) => {
            eprintln!("Unable to run 'git': {msg}");
            eprintln!();
            eprintln!("Possible causes:");
            eprintln!("- git is not installed or not on PATH");
            ExitCode::FAILURE
        }
        StagedChangesError::CommandFailed(msg) => {
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


