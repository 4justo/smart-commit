mod inspector;
mod repo_state;

use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

use crate::inspector::inspect_repository_state;

#[derive(Parser, Debug)]
struct Cli {
    // Optional target repo path; if omitted, use the current directory.
    #[arg(long)]
    repo: Option<PathBuf>,
}


fn main() -> ExitCode {
    let cli = Cli::parse();

    // If no path is provided, inspect the current working directory.
    let target = cli.repo.unwrap_or_else(|| {
        std::env::current_dir().expect("could not read current directory")
    });

    match inspect_repository_state(&target) {
        Ok(state) => {
            println!("Repository: valid");
            println!(
                "Branch: {}",
                state.branch.as_deref().unwrap_or("(detached or unknown)")
            );
            println!("HEAD: {}", state.head.as_deref().unwrap_or("(unknown)"));
            println!("Detached HEAD: {}", yes_no(state.detached_head));
            println!("Staged changes: {}", yes_no(state.has_staged_changes));
            println!("Unstaged changes: {}", yes_no(state.has_unstaged_changes));
            println!("Untracked files: {}", yes_no(state.has_untracked_files));
            println!("State: {}", safety_label(state.safety()));
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
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

fn safety_label(safety: repo_state::RepositorySafety) -> &'static str {
    match safety {
        repo_state::RepositorySafety::Safe => "ready",
        repo_state::RepositorySafety::Warning => "warning",
        repo_state::RepositorySafety::Blocked => "blocked",
    }
}
