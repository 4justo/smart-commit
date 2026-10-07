# SmartCommit MVP — Module-by-Module AI Development Prompts

## General Instruction for Every Module

You are helping build **SmartCommit**, a professional Git commit-planning and splitting tool written in Rust.

The developer is building and debugging the application incrementally.

Do not attempt to implement future modules early.

Before modifying code:

1. Inspect the existing repository.
2. Read `README.md`, `AI_DEVELOPMENT_CONTEXT.md`, `ARCHITECTURE.md`, and `PROJECT_STATE.md` if they exist.
3. Treat the repository as the source of truth.
4. Preserve working code.
5. Make the smallest change required for the current module.
6. Do not redesign previously completed modules without a demonstrated problem.
7. Add tests for the functionality introduced.
8. Do not perform destructive Git operations against the developer's actual repository.
9. Use temporary Git repositories for integration testing.
10. Explain important implementation decisions.

The MVP must remain usable without AI or an external LLM.

The MVP architecture should keep the core engine independent from the CLI.

---

# MODULE 0 — Rust Workspace Foundation

## Goal

Create the initial Rust workspace for SmartCommit.

Do not implement Git behavior yet.

## Requirements

Create a Cargo workspace suitable for gradual expansion.

Suggested initial structure:

```text
smartcommit/
├── crates/
│   ├── smartcommit-core/
│   └── smartcommit-cli/
├── tests/
├── fixtures/
├── docs/
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── PROJECT_STATE.md
└── AI_DEVELOPMENT_CONTEXT.md
```

`smartcommit-core` must be a library crate.

`smartcommit-cli` must be a binary crate depending on `smartcommit-core`.

Do not introduce additional crates unless they are actually needed.

## Initial Dependencies

Evaluate and use only what is currently necessary.

Likely dependencies:

```text
clap
thiserror
anyhow
serde
toml
```

Do not add `tokio`, `ratatui`, Tree-sitter, AI SDKs, or databases yet.

## Core Types

Define only basic shared types if necessary.

Possible examples:

```rust
pub struct RepositoryInfo;
pub struct ChangedFile;
pub struct CommitMessage;
pub struct CommitPlan;
```

Do not fill these with speculative fields.

## CLI

Create:

```bash
smartcommit --version
smartcommit --help
```

The binary should build and run.

## Testing

Add:

- workspace build verification
- basic CLI invocation test if practical

## Completion Criteria

The module is complete when:

```bash
cargo build --workspace
cargo test --workspace
```

both succeed.

Update `PROJECT_STATE.md` after completion.

Do not start repository analysis yet.

---

# MODULE 1 — Git Repository Detection

## Goal

Detect whether the current directory or a supplied path belongs to a Git repository.

## Requirements

Implement repository discovery in `smartcommit-core`.

The module should support:

```text
Current directory
Explicit --repo path
Nested directory inside repository
Repository root detection
```

Return structured information rather than printing directly.

Possible API:

```rust
Repository::discover(path)
```

or equivalent.

The result should expose at least:

```text
Repository root
.git location where applicable
Whether repository is bare
```

The exact API should follow Rust conventions and the existing codebase.

## Error Cases

Handle:

```text
Path does not exist
Path is not accessible
Not a Git repository
Bare repository if unsupported
```

Do not panic for expected user errors.

## CLI Integration

Add:

```bash
smartcommit doctor
```

For now, `doctor` only needs to verify repository detection.

Example output:

```text
Repository detected
Root: /path/to/project
```

If not inside a repository:

```text
No Git repository found.
```

## Testing

Use temporary directories.

Test:

1. Valid repository root
2. Nested path inside repository
3. Non-repository directory
4. Invalid path

Do not test against the real SmartCommit repository.

## Completion Criteria

Repository discovery is reliable and independently tested.

Do not inspect staged changes yet.

---

# MODULE 2 — Repository State Inspector

## Goal

Determine whether SmartCommit can safely operate on the current repository.

## Requirements

Build a repository-state model.

Detect:

```text
Current branch
HEAD commit
Detached HEAD
Merge in progress
Rebase in progress
Cherry-pick in progress
Conflicts
Untracked files
Unstaged modifications
Staged modifications
```

If possible, also identify:

```text
Partially staged files
```

but do not overcomplicate this module if reliable detection requires later diff work.

## Suggested Model

Conceptually:

```rust
pub struct RepositoryState {
    pub branch: Option<String>,
    pub head: Option<Oid>,
    pub detached_head: bool,
    pub merge_in_progress: bool,
    pub rebase_in_progress: bool,
    pub cherry_pick_in_progress: bool,
    pub has_conflicts: bool,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub has_untracked_files: bool,
}
```

Adapt this to the implementation.

## Safety Classification

Create a result such as:

```text
Safe
Warning
Blocked
```

Examples:

```text
Merge conflict -> Blocked
Rebase in progress -> Blocked
No staged changes -> Warning/Not ready
Normal staged changes -> Safe
```

Do not automatically modify the repository.

## CLI

Expand:

```bash
smartcommit doctor
```

Example:

```text
Repository: valid
Branch: main
HEAD: a91f4c2

Staged changes: yes
Unstaged changes: yes
Untracked files: no

State: ready
```

## Tests

Create temporary repositories representing:

- clean repo
- staged changes
- unstaged changes
- untracked file
- detached HEAD where practical
- conflict/repository operation states where practical

## Completion Criteria

SmartCommit can determine whether it should proceed or refuse to operate.

Do not create commits yet.

---

# MODULE 3 — Staged Change Discovery

## Goal

Read all currently staged changes.

## Requirements

SmartCommit must inspect the Git index and identify:

```text
Added files
Modified files
Deleted files
Renamed files where detectable
```

Create a structured type.

Example:

```rust
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}
```

and:

```rust
pub struct ChangedFile {
    pub path: PathBuf,
    pub change_kind: ChangeKind,
}
```

Add more fields only when required.

## Important Rule

The MVP operates on **staged changes**.

Do not accidentally use unstaged working-tree changes as the source of truth.

## CLI

Add:

```bash
smartcommit analyze
```

For this module, output only the staged files.

Example:

```text
Staged changes: 4

Modified  src/auth.rs
Added     src/session.rs
Deleted   src/legacy.rs
Modified  README.md
```

If no staged changes:

```text
No staged changes found.
```

## Tests

Test:

- staged added file
- staged modified file
- staged deleted file
- multiple staged files
- unstaged file is not included
- untracked and unstaged file is not included

## Completion Criteria

The staged-file list exactly represents the Git index.

---

# MODULE 4 — Diff Extraction Engine

## Goal

Extract the staged diff for every changed file.

## Requirements

For every `ChangedFile`, obtain the diff between:

```text
HEAD
and
Git index
```

Do not read the unstaged working-tree diff.

Create representations for:

```text
File diff
Added lines
Removed lines
Diff hunks
Hunk header
Hunk body
```

Possible conceptual structures:

```rust
pub struct FileDiff {
    pub path: PathBuf,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
}

pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}
```

Keep the model practical.

## Binary Files

Detect binary files.

Do not attempt to treat binary content as text.

Example:

```text
assets/logo.png
Binary file changed
```

## CLI

Enhance:

```bash
smartcommit analyze
```

Possible output:

```text
src/auth.rs
  +18 -6
  3 hunks

README.md
  +7 -1
  1 hunk
```

Add verbose diff output only behind an option if needed.

## Tests

Test:

- text modification
- new file
- deleted file
- multiple hunks
- binary file
- line counts

## Completion Criteria

SmartCommit has an accurate in-memory model of staged changes.

---

# MODULE 5 — Multiline Commit Message Parser

## Goal

Convert developer-supplied multiline text into individual intended commit messages.

## Input Example

```text
Fix authentication

Improve dashboard UI

Update README

Add payment validation
```

## Output

```text
[
  "Fix authentication",
  "Improve dashboard UI",
  "Update README",
  "Add payment validation"
]
```

## Requirements

Support:

```text
Blank-line-separated messages
One-message-per-line mode where appropriate
Whitespace trimming
Windows and Unix line endings
Ignoring empty entries
Preserving message text
```

Do not use AI.

## Input Methods

For MVP, support at least one clean method, such as:

```bash
smartcommit plan --messages-file commits.txt
```

Potentially also:

```bash
smartcommit plan --message "..."
```

Do not build an interactive editor yet unless simple.

## Validation

Reject:

```text
Empty message input
Only whitespace
Duplicate messages optionally warn
```

Do not enforce Conventional Commits yet unless configured.

## Tests

Test:

- blank-line separated messages
- CRLF input
- leading/trailing spaces
- empty input
- one message
- several messages

## Completion Criteria

Message parsing is deterministic and thoroughly tested.

---

# MODULE 6 — Basic Deterministic File Matcher

## Goal

Match staged files to commit messages without AI.

This is intentionally a basic MVP matcher.

## Requirements

Use deterministic evidence such as:

```text
Filename words
Directory words
File extension
Diff keywords
Commit message keywords
```

Example:

```text
Message:
"Fix authentication"

File:
src/auth/login.rs

Evidence:
auth
login
authentication
```

Assign a score.

Possible scoring model:

```text
Filename keyword match
Directory keyword match
Diff keyword match
Exact token match
Stemmed/simple normalized match
```

Keep it understandable.

Do not introduce embeddings or LLM calls.

## Output

For every message-file pair:

```rust
MatchScore {
    message_id,
    file_id,
    score,
    reasons,
}
```

Example:

```text
Fix authentication
  src/auth/login.rs   0.92
  src/auth/jwt.rs     0.83
  README.md           0.04
```

## Ambiguity

Do not force obviously ambiguous files into a group.

Allow:

```text
Unassigned
Needs review
```

## Explainability

Store reasons such as:

```text
matched directory token "auth"
matched diff token "login"
```

## Tests

Create synthetic examples:

- obvious auth match
- obvious docs match
- ambiguous shared utility
- no relevant message
- multiple relevant messages

## Completion Criteria

The deterministic matcher produces repeatable scores and explanations.

---

# MODULE 7 — Commit Planner

## Goal

Turn parsed messages, staged files, diffs, and matching scores into a proposed set of commits.

## Requirements

Create a core planning model.

Conceptually:

```rust
pub struct CommitPlan {
    pub groups: Vec<CommitGroup>,
    pub unassigned: Vec<ChangedFile>,
}

pub struct CommitGroup {
    pub message: String,
    pub files: Vec<ChangedFile>,
    pub confidence: f32,
}
```

Adapt as necessary.

## Rules

The planner should:

1. Assign high-confidence files.
2. Avoid assigning one whole file to multiple commits in the MVP.
3. Leave ambiguous files unassigned.
4. Calculate group confidence.
5. Preserve the developer's message order initially.
6. Report empty commit groups.

Do not implement hunk-level splitting yet.

## Validation

Detect:

```text
Message with no matching files
File with no matching message
File matched almost equally to multiple messages
All files assigned
```

## CLI

Add:

```bash
smartcommit plan --messages-file commits.txt
```

Example:

```text
Commit Plan

[1] Fix authentication
    src/auth.rs
    src/login.rs
    Confidence: 92%

[2] Improve dashboard
    src/dashboard.rs
    Confidence: 88%

Unassigned:
    src/utils.rs
```

No Git modification should occur.

## Tests

Test:

- successful one-to-one grouping
- one message to several files
- empty group
- unassigned file
- ambiguity
- deterministic order

## Completion Criteria

A complete non-destructive commit plan can be produced.

---

# MODULE 8 — Timeline Engine

## Goal

Generate timestamps for proposed commits.

## Requirements

Implement MVP timeline modes:

### Fixed

Example:

```text
10-minute interval
```

### Random Range

Example:

```text
5–15 minutes
```

### Custom Start

Example:

```bash
--start "2026-09-01T09:00:00+03:00"
```

The timeline engine should produce timestamps for commit groups.

## Important Requirements

- Preserve chronological ordering.
- Never accidentally generate a future timestamp unless explicitly allowed.
- Make random mode testable by allowing a seeded RNG or deterministic test interface.
- Keep timeline generation separate from Git commit execution.

## Model

Conceptually:

```rust
pub struct PlannedCommitTime {
    pub commit_index: usize,
    pub timestamp: DateTime<FixedOffset>,
}
```

Use an appropriate date/time crate only if needed.

## Transparency

If timestamps are synthetic/reconstructed, label them accordingly in preview output.

## Tests

Test:

- fixed intervals
- random bounds
- ordering
- single commit
- custom start
- invalid range

## Completion Criteria

The timeline engine produces validated timestamps without creating commits.

---

# MODULE 9 — Safety Checkpoint Engine

## Goal

Create a reliable recovery point before SmartCommit performs commit operations.

## Requirements

Before commit execution:

Capture:

```text
Original HEAD
Current branch
Repository state
Staged changes
Relevant metadata required for recovery
```

Create a clearly named backup reference.

Possible form:

```text
refs/smartcommit/backups/<timestamp>
```

or a backup branch.

Choose the safer and cleaner Git approach after evaluating both.

Do not clutter normal branch listings unnecessarily if a custom ref is more appropriate.

## Checkpoint Record

Create a structured record such as:

```rust
pub struct SafetyCheckpoint {
    pub original_head: Oid,
    pub branch: Option<String>,
    pub backup_ref: String,
}
```

## Rules

A checkpoint must be created before the first commit operation.

If checkpoint creation fails:

```text
Abort.
Do not modify repository history.
```

## Tests

Temporary repositories only.

Verify:

- backup points to correct HEAD
- backup remains valid after commits
- failure does not modify repository
- checkpoint naming collision handling

## Completion Criteria

SmartCommit can safely reference the pre-operation state.

---

# MODULE 10 — Commit Executor

## Goal

Create multiple commits from an approved `CommitPlan`.

This is the most safety-critical MVP module.

## Requirements

For each commit group:

1. Ensure only the intended files are staged for that commit.
2. Create the commit.
3. Apply the planned author/committer timestamp if timeline mode is enabled.
4. Continue to the next group.
5. Verify the commit was created successfully.

The original working-tree content must remain intact.

## Critical Considerations

SmartCommit must correctly handle repositories where:

```text
staged changes exist
and
unstaged changes may also exist
```

Do not destroy unstaged changes.

Do not blindly run:

```bash
git reset --hard
```

Do not use destructive commands merely because they simplify implementation.

Evaluate whether to manipulate:

```text
Git index
temporary index
libgit2 index
GIT_INDEX_FILE
```

The safest architecture is preferred.

## Failure Behavior

If commit #3 of 5 fails:

```text
Stop immediately.
Report completed commits.
Report failure.
Provide checkpoint recovery instructions.
```

Do not silently proceed.

## Hooks

Be aware that Git hooks may reject a commit.

Treat hook failures as valid execution failures.

## Commit Metadata

Support:

```text
Message
Author date
Committer date
```

Do not modify author identity unless explicitly configured.

## Tests

Use temporary repositories.

Test:

- two commits from two file groups
- three commits
- timestamps
- unstaged changes preserved
- staged content correct
- failure midway
- hook failure if practical
- deleted files
- added files

## Completion Criteria

Multiple commits can be created safely and exactly match the approved plan.

---

# MODULE 11 — Preview and Confirmation CLI

## Goal

Provide a professional user-facing workflow around the core modules.

## Requirements

Implement a command such as:

```bash
smartcommit plan --messages-file commits.txt
```

and:

```bash
smartcommit commit --messages-file commits.txt
```

or another clean CLI structure consistent with the current codebase.

## Preview

Show:

```text
Repository
Branch
Number of staged files
Number of proposed commits
Messages
Files per commit
Confidence
Unassigned files
Planned timestamps
Warnings
```

Example:

```text
SMARTCOMMIT PLAN

Repository: /home/user/project
Branch: main

4 staged files
3 proposed commits

[1] fix(auth): validate JWT
    src/auth.rs
    src/jwt.rs
    Confidence: 93%
    Time: 10:15

[2] docs: update README
    README.md
    Confidence: 99%
    Time: 10:27

Unassigned:
    src/utils.rs

Warnings:
    1 file requires manual review
```

## Confirmation

Before committing:

```text
Create these commits? [y/N]
```

Default must be **No**.

Support:

```bash
--yes
```

for explicit non-interactive execution.

## Dry Run

Support:

```bash
smartcommit commit --dry-run
```

It must perform every validation and planning step but make no Git modifications.

## Exit Codes

Use meaningful non-zero exit codes for failures.

## Completion Criteria

The MVP can be used safely from the terminal without internal knowledge.

---

# MODULE 12 — Restore and Recovery

## Goal

Allow the developer to recover from a SmartCommit operation.

## Requirements

Implement:

```bash
smartcommit restore
```

or an equivalent explicit command.

The restore system should use the safety checkpoint created before execution.

## Safety

Before restoring:

Show:

```text
Current HEAD
Checkpoint HEAD
Branch
Commits that would be removed/replaced
Working-tree state
```

Require confirmation.

Do not destroy unrelated work created after the SmartCommit operation.

If automatic restoration is unsafe:

```text
Refuse automatic restore and provide exact manual recovery instructions.
```

## Tracking

Consider storing operation metadata under:

```text
.git/smartcommit/
```

Example:

```text
.git/smartcommit/last-operation.toml
```

Store:

```text
Checkpoint ref
Original HEAD
Final HEAD
Created commits
Operation timestamp
```

Do not store secrets or unnecessary source code.

## Tests

Test:

- successful restore
- operation metadata
- restore after multiple commits
- changed working tree after operation
- unsafe restore refusal

## Completion Criteria

The user has a credible recovery path.

---

# MODULE 13 — Configuration

## Goal

Introduce minimal MVP configuration without overengineering.

## File

Potential:

```text
.smartcommit.toml
```

## MVP Options

Support only currently implemented options.

Example:

```toml
[matching]
minimum_confidence = 0.70

[timeline]
mode = "fixed"
gap_minutes = 10

[commit]
require_confirmation = true
```

Do not add configuration for future AI features yet.

## Resolution Order

Define predictable precedence:

```text
CLI option
>
project configuration
>
default
```

## Validation

Invalid configuration must produce readable errors.

## Tests

Test:

- default config
- valid file
- invalid TOML
- invalid range
- CLI overrides config

## Completion Criteria

MVP behavior is configurable without unnecessary complexity.

---

# MODULE 14 — End-to-End MVP Test Suite

## Goal

Prove the complete MVP works safely.

## Requirements

Create end-to-end tests using temporary Git repositories.

Do not use the SmartCommit repository itself.

## Main Scenario

Prepare:

```text
src/auth.rs
src/dashboard.rs
README.md
```

Stage all changes.

Provide:

```text
Fix authentication

Improve dashboard

Update documentation
```

Run SmartCommit.

Verify resulting history contains three commits and that each commit contains the expected file changes.

## Additional Scenarios

Test:

1. No repository
2. No staged changes
3. One message
4. Multiple messages
5. Ambiguous file
6. Unassigned file
7. Added file
8. Deleted file
9. Unstaged changes preserved
10. Timeline timestamps
11. Dry run
12. User cancellation
13. Mid-operation failure
14. Recovery checkpoint
15. Restore behavior
16. Binary file
17. Invalid config

## Completion Criteria

The MVP should pass:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

No MVP release should occur until these succeed.

---

# MODULE 15 — MVP Polish and Release Preparation

## Goal

Make v0.1 professional enough for an initial public release.

## Requirements

Review:

```text
CLI consistency
Error messages
Documentation
Logging
Tests
Repository safety
Cross-platform assumptions
Dependency list
Configuration
Recovery behavior
```

## Documentation

Update:

```text
README.md
ARCHITECTURE.md
PROJECT_STATE.md
CHANGELOG.md
SECURITY.md
```

Document only implemented features as available.

Future functionality must remain clearly marked as planned.

## CLI Help

Ensure:

```bash
smartcommit --help
smartcommit analyze --help
smartcommit plan --help
smartcommit commit --help
smartcommit restore --help
smartcommit doctor --help
```

are clear and professional.

## Release Checklist

Verify:

```text
cargo fmt
cargo clippy
cargo test
release build
Linux behavior
Windows path handling
macOS assumptions
README examples
recovery procedure
```

## Completion Criteria

Tag:

```text
v0.1.0
```

only when the core splitter is reliable enough to be tested by real users.

---

# MVP BUILD ORDER

Do not skip around unnecessarily.

Recommended order:

```text
Module 0
Workspace foundation
      |
      v
Module 1
Repository detection
      |
      v
Module 2
Repository state
      |
      v
Module 3
Staged changes
      |
      v
Module 4
Diff extraction
      |
      v
Module 5
Message parser
      |
      v
Module 6
Basic matcher
      |
      v
Module 7
Commit planner
      |
      v
Module 8
Timeline engine
      |
      v
Module 9
Safety checkpoint
      |
      v
Module 10
Commit executor
      |
      v
Module 11
CLI workflow
      |
      v
Module 12
Restore
      |
      v
Module 13
Configuration
      |
      v
Module 14
End-to-end tests
      |
      v
Module 15
v0.1 release polish
```

---

# RULE FOR MOVING TO THE NEXT MODULE

Before moving forward, the AI must report:

```text
MODULE COMPLETION REPORT

Module:
[Module name]

Implemented:
[...]

Files changed:
[...]

Tests added:
[...]

Commands verified:
[...]

Known limitations:
[...]

Technical debt:
[...]

PROJECT_STATE.md updated:
Yes/No

Ready for next module:
Yes/No
```

If tests fail, fix the current module before beginning another one.

---

# MVP DEFINITION OF DONE

The MVP is complete when a developer can:

```bash
git add .
```

provide several intended commit messages, run SmartCommit, review a proposed grouping and timeline, approve it, and receive multiple independent commits without losing working-tree changes.

The MVP must provide:

```text
Repository detection
Repository safety inspection
Staged file detection
Staged diff analysis
Multiline message parsing
Basic deterministic semantic matching
Commit planning
Confidence and ambiguity reporting
Timeline generation
Preview
Dry run
Developer confirmation
Safety checkpoint
Multiple commit creation
Timestamp application
Failure handling
Recovery metadata
Restore mechanism
Configuration
Automated tests
Professional CLI errors
```

The MVP explicitly does NOT require:

```text
LLMs
Embeddings
Tree-sitter
Hunk-level splitting
Automatic commit generation
Semantic history search
Bug tracing
VS Code integration
GitHub integration
TUI
```

Those belong to later releases.

The priority for v0.1 is:

> **A small, safe, reliable Git engine is more valuable than a large intelligent system that cannot be trusted.**

---

# MODULE 15 — AI Integration Foundation

## Goal

Add AI provider infrastructure for commit message generation and semantic analysis.

## Requirements

Implement AI provider trait with support for:
- Claude (Anthropic)
- OpenAI
- Google Gemini
- Local LLMs (Ollama)

## Implementation

- Create `AIProvider` trait with `generate_message()` method
- Implement providers for Claude, OpenAI, Gemini, and Local LLM
- Add configuration via environment variables and `.smc.toml`
- Implement fallback mechanism between providers
- Ensure AI features are optional (feature-gated)

---

# MODULE 16 — AI Commit Message Generation

## Goal

Use AI to automatically generate conventional commit messages from code changes.

## Requirements

- Analyze staged diffs and generate commit messages
- Support conventional commit format (type: description)
- Generate multiple atomic commit messages
- Fall back to keyword matching if AI fails

---

# MODULE 17 — AI Semantic File Matching

## Goal

Use AI to semantically match files to commit messages based on actual code meaning.

## Requirements

- Analyze file changes and commit messages
- Match files to appropriate commits
- Combine with deterministic keyword matching
- Provide confidence scores

---

# MODULE 18 — Code Structure Detection

## Goal

Enhance structural understanding of source code by detecting code structures using Tree-sitter.

## Requirements

- Detect Functions, Methods, Classes, Structs, Interfaces, Modules, Imports, Exports, Traits, Enums
- Map changed hunks to affected structures
- Support multiple languages (Rust, Python, JavaScript/TypeScript, PHP, Java, C#, Go, C, C++)
- Produce structured results with language, type, name, file, line_range
- Ensure unsupported languages fall back to generic diff analysis
- Parser failures never crash SmartCommit

## Implementation

- Add `StructureInfo` with `file` field
- Update all language parsers to pass file paths
- Change `DiffHunk.structure` to `DiffHunk.structures` (Vec) for multiple structures
- Implement hunk-to-structure mapping with overlap detection
- Add AI-assisted structure analysis for ambiguous cases

---

# MODULE 19 — Relationship and Dependency Analysis

## Goal

Analyze relationships between files, hunks, code structures, tests, and dependencies.

## Requirements

- Detect imports and module references.
- Detect symbol references.
- Identify shared classes/functions.
- Detect shared directories.
- Map tests to implementations.
- Analyze configuration dependencies.
- Analyze package dependencies.
- Create a structured relationship graph.
- AI may suggest relationships but deterministic and AI evidence must remain distinguishable.

## Implementation

- Test-to-implementation mapping via naming conventions
- Import/module reference analysis for Rust, Python, JavaScript/TypeScript
- Shared directory analysis
- Relationship graph with `RelationshipType` (TestCoverage, Import, SharedDirectory)
- `EvidenceSource` (Deterministic only - AI evidence separate)
- Integration with planner to enhance commit grouping

---

# POST-MVP FEATURES

## Installation & Distribution

- Pre-built binaries for Linux, macOS (Apple Silicon & Intel), Windows
- GitHub Actions release workflow
- Simple download-and-use experience
- No coding required for end users

## AI Providers

- Claude (Anthropic)
- OpenAI
- Google Gemini
- Local LLMs (Ollama)
- Configurable via environment variables or `.smc.toml`

## Configuration

- `.smc.toml` for default behavior
- AI provider selection
- Model configuration
- Generation parameters (max_tokens, temperature)
- Matching thresholds
- Timeline settings
