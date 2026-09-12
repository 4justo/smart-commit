# SmartCommit

SmartCommit is a professional Git commit-planning and splitting
tool, written in Rust. It lets a developer stage a batch of related
changes, describe the several logical commits hidden inside that
batch in plain text, and have SmartCommit propose — and, once
approved, create — those commits safely.

**Status: MVP core loop works (through Module 11 of the v0.1 build
plan).** The tool can plan and create real, correctly-split commits
end to end. See "Currently implemented" below for exactly what that
covers, and what's still ahead.

## Currently implemented

```bash
smartcommit doctor      # repository detection + full state (branch, HEAD, conflicts, staged/unstaged/untracked)
smartcommit analyze     # staged files with diff summaries (--verbose for full hunks)
smartcommit plan        # full preview: proposed commits, confidence, timestamps, unassigned files, warnings
smartcommit commit      # plan, confirm ([y/N], or --yes), and create real commits (or --dry-run)
```

The full workflow below is real and tested, not aspirational.

Still ahead: `smartcommit restore` (undo/recovery — Module 12),
`.smartcommit.toml` configuration (Module 13), and release polish
(Modules 14-15). See `PROJECT_STATE.md` for exact per-module status.

## Workflow

```bash
git add .

# preview only, makes no Git modification
smartcommit plan --messages-file commits.txt

# review the proposed grouping and timeline, then create the commits
smartcommit commit --messages-file commits.txt
# or skip the confirmation prompt:
smartcommit commit --messages-file commits.txt --yes
# or see the full preview without committing anything:
smartcommit commit --messages-file commits.txt --dry-run
```

`commits.txt` is blank-line-separated commit messages:

```text
Fix authentication in login module

Improve dashboard widgets
```

### Timeline options (both `plan` and `commit`)

```bash
--start <RFC3339>              # custom start time for the first commit, e.g. 2026-09-01T09:00:00+03:00
--interval-minutes <N>         # fixed gap between commits (default: 10)
--random-min-minutes <N>       # random gap instead of fixed — must be given with --random-max-minutes
--random-max-minutes <N>
--allow-future                 # permit a computed timestamp after "now" (only relevant with --start)
```

With no `--start`, the *last* commit is anchored at the current time
and earlier commits are placed backward — this can never produce a
future timestamp.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation/repository failure (not a repo, blocked state, no staged changes, bad input) |
| 2 | User declined the confirmation prompt |
| 3 | Execution failed partway through (some commits were created — see the report and recovery checkpoint) |

## Building

```bash
cargo build --workspace
cargo test --workspace
```

## Project layout

```text
smartcommit/
├── crates/
│   ├── smartcommit-core/   # Git engine + planning logic (library)
│   └── smartcommit-cli/    # CLI binary, depends on smartcommit-core
├── tests/                  # workspace-level integration tests (future modules)
├── fixtures/                # test fixtures (future modules)
├── docs/
│   ├── modules.md          # the module-by-module build spec (source of truth)
│   └── testing/            # per-module manual testing guides
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md          # full architectural spec + implementation-status addendum
└── PROJECT_STATE.md         # exact per-module progress
```

`smartcommit-core` must remain usable without AI or any external
LLM, and independent of the CLI.
