# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-26

### Added
- **Repository Detection** (Module 1)
  - Detect Git repositories from any path within a repository
  - Support for nested directories
  - Bare repository detection
  - `scm doctor` command for repository diagnostics

- **Repository State Inspection** (Module 2)
  - Full repository state analysis (branch, HEAD, detached HEAD)
  - Detection of in-progress operations (merge, rebase, cherry-pick)
  - Conflict detection
  - Staged/unstaged/untracked file tracking
  - Safety classification (Safe/Warning/Blocked)

- **Staged Change Discovery** (Module 3)
  - List all staged files with change types (Added/Modified/Deleted/Renamed)
  - Rename detection with old path tracking
  - `scm analyze` command for staged file inspection

- **Diff Extraction Engine** (Module 4)
  - Extract unified diffs for all staged changes
  - Hunk-level diff parsing
  - Binary file detection
  - Line addition/deletion counts
  - `--verbose` flag for full diff content

- **Multiline Commit Message Parser** (Module 5)
  - Blank-line-separated message parsing
  - Cross-platform line ending support (CRLF/LF)
  - Whitespace trimming
  - Empty entry filtering

- **Deterministic File Matcher** (Module 6)
  - Token-based file-to-message matching
  - Filename, directory, and diff keyword analysis
  - Confidence scoring with explainable reasons
  - Deterministic results across runs

- **Commit Planner** (Module 7)
  - Automatic file grouping by commit message
  - Confidence-based assignment with ambiguity detection
  - Unassigned file handling
  - Warning generation for edge cases
  - `scm plan` command for commit preview

- **Timeline Engine** (Module 8)
  - Fixed-interval timestamp generation
  - Random-range timestamp generation
  - Custom start time support
  - Future-timestamp prevention
  - Chronological ordering guarantees

- **Safety Checkpoint Engine** (Module 9)
  - Pre-operation state capture
  - Git ref-based recovery points
  - Repository state snapshots
  - Collision-safe ref naming
  - No repository modification on failure

- **Commit Executor** (Module 10)
  - Multi-commit creation from approved plans
  - Temporary index per commit (preserves real index)
  - Git hook support (pre-commit, etc.)
  - Planned timestamp application
  - Failure handling with partial execution reporting
  - Unstaged change preservation

- **Preview and Confirmation CLI** (Module 11)
  - Full commit plan preview
  - Interactive confirmation prompts ([y/N], default No)
  - `--yes` flag for non-interactive mode
  - `--dry-run` for preview without execution
  - Meaningful exit codes (0=success, 1=validation failure, 2=user declined, 3=partial failure)
  - Timeline CLI flags (`--start`, `--interval-minutes`, `--random-min-max-minutes`, `--allow-future`)

- **Restore and Recovery** (Module 12)
  - Operation metadata tracking (`.git/smartcommit/last-operation.toml`)
  - Automatic restore when safe
  - Safety checks before restore (HEAD, branch, checkpoint integrity)
  - Manual recovery instructions for unsafe scenarios
  - `scm restore` command

- **Configuration** (Module 13)
  - `.smc.toml` configuration file support
  - Matching parameters (minimum confidence, ambiguity margin)
  - Timeline settings (fixed/random intervals)
  - Commit behavior (confirmation requirement)
  - CLI flag → config → default resolution order

- **End-to-End Test Suite** (Module 14)
  - 17 comprehensive integration test scenarios
  - Full pipeline testing (plan → checkpoint → execute → restore)
  - Edge case coverage (no repo, no changes, conflicts, binary files, etc.)

- **Hunk-Level Change Analysis** (Module 16)
  - Extended `DiffHunk` with deterministic IDs
  - Parsed hunk ranges (old_start, old_count, new_start, new_count)
  - Hunk statistics (additions, deletions, context, total changed lines)
  - Raw content preservation
  - Per-hunk summary in `scm analyze --verbose`

### Changed
- CLI binary renamed from `smartcommit` to `scm` (Module 16)

### Security
- No secrets or source code stored in operation records
- Temporary index files isolated from real index
- Checkpoint refs stored outside normal branch namespace
- Safety checks before all destructive operations

### Documentation
- Comprehensive README with workflow examples
- Full architecture specification (ARCHITECTURE.md)
- Per-module implementation tracking (PROJECT_STATE.md)
- Module-by-module build guide (docs/modules.md)

### Testing
- 144 passing tests (12 CLI + 17 end-to-end + 115 core)
- All tests use temporary Git repositories
- Deterministic testing (no reliance on wall-clock or random seeds)
- Cross-platform compatibility verified

## [0.1.0] - Release Notes

SmartCommit v0.1.0 is a minimal, safe, and reliable Git commit planning and splitting tool.

**Core Philosophy**: A small, safe, reliable Git engine is more valuable than a large intelligent system that cannot be trusted.

**What's Included**:
- Deterministic file-to-commit matching without AI
- Safe multi-commit creation with full recovery
- Timeline generation for realistic commit histories
- Configuration support for custom workflows
- Comprehensive test coverage

**What's Not Included** (Planned for Future Releases):
- AI-powered commit message generation
- AI semantic file matching
- Hunk-level commit splitting
- Code structure detection (Tree-sitter)
- VS Code integration

**Installation**:
```bash
# Build from source
cargo build --workspace --release
cargo install --path crates/smartcommit-cli
```

**Quick Start**:
```bash
git add .
cat > commits.txt <<EOF
Fix authentication
Improve dashboard
EOF
scm plan --messages-file commits.txt
scm commit --messages-file commits.txt
```

**Safety Features**:
- Automatic safety checkpoints before all operations
- Restore mechanism for recovery
- Hook support (pre-commit, etc.)
- Unstaged change preservation
- No repository modification on preview or dry-run

**Platform Support**:
- Linux
- macOS (Apple Silicon & Intel)
- Windows

See README.md for full documentation.
