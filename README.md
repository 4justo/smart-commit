# SmartCommit

SmartCommit is a professional Git commit-planning and splitting tool,
written in Rust. It lets a developer stage a batch of related changes,
describe the several logical commits hidden inside that batch in plain
text, and have SmartCommit propose — and, once approved, create — those
commits safely.

**Status: MVP complete through Module 14 (end-to-end test suite).**
The full pipeline — planning, configuration, committing, restore, and
automated testing — is implemented and passing. Module 15 (release
polish) is the only remaining step before v0.1.0.

## Currently implemented

```bash
smartcommit doctor      # repository detection + full state (branch, HEAD, conflicts, staged/unstaged/untracked)
smartcommit analyze     # staged files with diff summaries (--verbose for full hunks)
smartcommit plan        # full preview: proposed commits, confidence, timestamps, unassigned files, warnings
smartcommit commit      # plan, confirm ([y/N], or --yes), and create real commits (or --dry-run)
smartcommit restore     # undo the most recent SmartCommit operation, if it's safe to do so automatically
```

All commands above are real and tested end to end.

## Workflow

```bash
git add .

# preview only — makes no Git modification
smartcommit plan --messages-file commits.txt

# review the proposed grouping and timeline, then create the commits
smartcommit commit --messages-file commits.txt
# or skip the confirmation prompt:
smartcommit commit --messages-file commits.txt --yes
# or preview without committing anything:
smartcommit commit --messages-file commits.txt --dry-run
```

If you need to undo it afterward:

```bash
smartcommit restore
# or, non-interactively:
smartcommit restore --yes
```

Restore only proceeds automatically when nothing risky has happened
since the operation (no new commits on the branch, the branch hasn't
changed, the safety checkpoint is still intact). Otherwise it refuses
and prints the exact manual command instead.

`commits.txt` is a blank-line-separated list of commit messages:

```text
Fix authentication in login module

Improve dashboard widgets
```

### Timeline options (both `plan` and `commit`)

```bash
--start <RFC3339>              # custom start time for the first commit
                               # e.g. 2026-09-01T09:00:00+03:00
--interval-minutes <N>         # fixed gap between commits (default: 10)
--random-min-minutes <N>       # random gap instead of fixed —
--random-max-minutes <N>       # both flags must be given together
--allow-future                 # permit a timestamp after "now"
                               # (only relevant when --start is given)
```

With no `--start`, the *last* commit is anchored at the current time
and earlier commits are placed backward — this can never produce a
future timestamp.

### Configuration

Place a `.smartcommit.toml` at your repository root to tune default
behaviour without passing flags every time:

```toml
[matching]
minimum_confidence = 0.70   # min score for a file to be assigned (default 0.3)
ambiguity_margin   = 0.05   # margin below which two candidates are "too close" (default 0.1)

[timeline]
gap_minutes = 15            # fixed interval in minutes (default 10)
# or for random gaps:
# random_min_minutes = 5
# random_max_minutes = 30

[commit]
require_confirmation = true # set to false to skip the [y/N] prompt by default
```

All sections and fields are optional. Missing fields fall back to the
built-in defaults. CLI flags always override config values.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation/repository/config failure |
| 2 | User declined the confirmation prompt |
| 3 | Execution failed partway (some commits were created — see the report and recovery checkpoint) |

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
│   └── smartcommit-cli/    # CLI binary + integration tests
│       └── tests/
│           ├── cli_invocation.rs   # per-module CLI tests (Modules 0–12)
│           └── end_to_end.rs       # Module 14 full MVP test suite
├── docs/
│   ├── modules.md          # module-by-module build spec (source of truth)
│   └── testing/            # per-module manual testing guides
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md         # full architectural spec + implementation-status addendum
└── PROJECT_STATE.md        # exact per-module progress
```

`smartcommit-core` remains usable without AI or any external LLM,
and is independent of the CLI.
