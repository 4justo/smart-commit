# SmartCommit Architecture

> Technical architecture and design specification for SmartCommit.

**Project:** SmartCommit  
**Language:** Rust  
**Architecture:** Modular CLI-first application  
**Current Target:** MVP / v0.1  
**Status:** Early Development

---

# 1. Overview

SmartCommit is a Rust-based developer tool designed to analyze staged Git changes and transform logically unrelated changes into multiple, meaningful Git commits.

The MVP focuses on deterministic Git analysis and safe commit creation.

The system is designed so that advanced AI functionality can be introduced later without rewriting the Git engine.

The fundamental workflow is:

```text
Developer
    |
    v
SmartCommit CLI
    |
    v
Core Engine
    |
    +-------------------+
    |                   |
    v                   v
Git Analysis       Commit Planning
    |                   |
    +---------+---------+
              |
              v
        Safety Validation
              |
              v
          Preview
              |
              v
      Developer Approval
              |
              v
       Commit Execution
              |
              v
          Verification
```

---

# 2. Architectural Goals

SmartCommit architecture must prioritize:

1. Safety
2. Correctness
3. Predictability
4. Testability
5. Separation of concerns
6. Developer control
7. Explainability
8. Extensibility
9. Cross-platform compatibility
10. Minimal complexity

The architecture must avoid premature implementation of future features.

---

# 3. MVP Scope

The MVP is responsible for:

```text
Repository discovery
Repository state inspection
Staged change detection
Staged diff extraction
Multiline commit message parsing
Deterministic file/message matching
Commit planning
Confidence scoring
Timeline generation
Preview
Developer confirmation
Safety checkpoint
Multiple commit creation
Failure handling
Recovery metadata
Restore
Configuration
Testing
```

The MVP does not require:

```text
LLM integration
Embeddings
Vector databases
Tree-sitter
Hunk-level semantic splitting
Semantic Git history search
Bug tracing
VS Code extension
GitHub integration
Web UI
TUI
```

These belong to later versions.

---

# 4. Core Architectural Principle

The most important architectural rule is:

> **The Git engine must not depend on the CLI.**

The CLI is an interface.

The core engine contains the actual application logic.

This allows future interfaces to use the same engine:

```text
                    SmartCommit Core
                          |
             +------------+------------+
             |            |            |
             v            v            v
            CLI          TUI       VS Code
```

The core must never assume that output is going directly to a terminal.

---

# 5. High-Level Architecture

```text
+-------------------------------------------------------+
|                    SmartCommit CLI                    |
|                                                       |
| Commands | Arguments | Output | Confirmation | Exit |
+----------------------------+--------------------------+
                             |
                             v
+-------------------------------------------------------+
|                  SmartCommit Core                     |
|                                                       |
| Repository | State | Diff | Matching | Planning      |
| Timeline   | Safety | Execution | Recovery           |
+----------------------------+--------------------------+
                             |
                             v
+-------------------------------------------------------+
|                       Git Layer                       |
|                                                       |
| Repository | HEAD | Index | Working Tree | Commits    |
+-------------------------------------------------------+
```

The CLI should only translate user input into core operations and render results.

---

# 6. Rust Workspace

The initial workspace should be deliberately small.

```text
smartcommit/
│
├── crates/
│   ├── smartcommit-core/
│   └── smartcommit-cli/
│
├── tests/
├── fixtures/
├── docs/
│
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
├── PROJECT_STATE.md
├── AI_DEVELOPMENT_CONTEXT.md
├── CONTRIBUTING.md
├── SECURITY.md
└── CHANGELOG.md
```

Future modules can be extracted into separate crates only when there is a demonstrated architectural reason.

---

# 7. Crate Responsibilities

## 7.1 smartcommit-core

The core library contains all application logic.

Responsibilities include:

```text
Repository discovery
Repository state
Git changes
Diff parsing
Message parsing
Matching
Commit planning
Timeline generation
Safety
Commit execution
Recovery
Configuration
Validation
```

It must not contain terminal-specific rendering.

It should be usable by another application without requiring the CLI.

---

## 7.2 smartcommit-cli

The CLI is responsible for:

```text
Argument parsing
Command routing
Terminal output
Interactive confirmation
Formatting
Exit codes
User-facing errors
```

It should call `smartcommit-core`.

It should not implement Git logic directly.

Bad:

```text
CLI
 |
 +-- runs git commands
 +-- parses diff
 +-- creates commits
```

Good:

```text
CLI
 |
 v
Core
 |
 +-- Git
 +-- Diff
 +-- Planner
 +-- Executor
```

---

# 8. Internal Core Modules

The MVP core should conceptually contain these components:

```text
smartcommit-core
│
├── repository
├── state
├── changes
├── diff
├── messages
├── matching
├── planner
├── timeline
├── safety
├── execution
├── recovery
├── config
└── validation
```

These can initially be Rust modules within one crate.

They do not need to become separate crates.

---

# 9. Repository Module

## Responsibility

The repository module discovers and represents the Git repository.

It should answer:

```text
Where is the repository?
What is its root?
Is it valid?
Is it bare?
```

Conceptual API:

```rust
Repository::discover(path)
```

The repository object should become the foundation for other Git operations.

---

# 10. Repository State Module

## Responsibility

Determine whether the repository is safe for SmartCommit operations.

It should detect:

```text
Current branch
HEAD
Detached HEAD
Merge state
Rebase state
Cherry-pick state
Conflicts
Staged changes
Unstaged changes
Untracked files
```

Conceptual model:

```rust
RepositoryState
```

The state module must not modify the repository.

It only observes and reports.

---

# 11. Change Discovery Module

## Responsibility

Determine what is staged in the Git index.

The MVP operates on:

```text
HEAD -> Index
```

not:

```text
HEAD -> Working Tree
```

This distinction is critical.

Example:

```text
Working Tree
     |
     +-- unstaged changes
     |
     v
Git Index
     |
     +-- staged changes
     |
     v
HEAD
```

SmartCommit should operate only on the staged changes unless a future feature explicitly expands this behavior.

---

# 12. Diff Module

## Responsibility

Represent staged differences in a structured form.

The diff module should understand:

```text
Added files
Modified files
Deleted files
Renamed files
Binary files
Diff hunks
Added lines
Deleted lines
Context lines
```

Conceptual model:

```rust
FileDiff {
    path,
    change_kind,
    hunks,
    additions,
    deletions,
}
```

The diff module should not decide which commit a file belongs to.

That is the planner/matching layer's responsibility.

---

# 13. Message Module

## Responsibility

Convert user-provided commit text into structured commit messages.

Example input:

```text
Fix authentication

Improve dashboard

Update documentation
```

Becomes:

```text
CommitMessage
CommitMessage
CommitMessage
```

The message module must remain deterministic.

AI is not involved in MVP message parsing.

---

# 14. Matching Module

## Responsibility

Determine how strongly a changed file relates to a commit message.

MVP matching must be deterministic.

Possible evidence:

```text
Filename
Directory
File extension
Changed symbols where available
Diff text
Keywords
Normalized tokens
```

Example:

```text
Message:
Fix authentication

File:
src/auth/login.rs

Score:
0.92

Reasons:
- "auth" matched directory
- "login" matched filename
- authentication-related diff content
```

The matcher returns evidence.

It does not create commits.

---

# 15. Matching Model

Conceptually:

```rust
MatchScore {
    message_id,
    file_id,
    score,
    reasons,
}
```

Scores must be deterministic for the same input.

This is important for:

- testing
- debugging
- reproducibility
- user trust

---

# 16. Ambiguity Handling

SmartCommit must not force every file into a commit.

Example:

```text
Message A -> 0.51
Message B -> 0.49
```

This is ambiguous.

The planner should report:

```text
File requires review.
```

rather than pretending the decision is certain.

Possible states:

```text
Assigned
Ambiguous
Unassigned
Ignored
```

---

# 17. Commit Planner

## Responsibility

The planner converts analysis results into an executable plan.

Input:

```text
Repository state
Changed files
Diffs
Commit messages
Match scores
```

Output:

```text
CommitPlan
```

Conceptually:

```rust
CommitPlan {
    groups,
    unassigned,
    warnings,
}
```

Each group contains:

```text
Commit message
Files
Confidence
Reasons
```

The planner must not modify Git.

---

# 18. Planning Pipeline

```text
Staged Files
     |
     v
Diff Extraction
     |
     v
Message Parsing
     |
     v
Matching
     |
     v
Scoring
     |
     v
Ambiguity Detection
     |
     v
Commit Planner
     |
     v
CommitPlan
```

The output is purely informational until passed to the execution layer.

---

# 19. Commit Plan as a First-Class Object

The `CommitPlan` is one of the most important concepts in SmartCommit.

It separates:

```text
What SmartCommit thinks should happen
```

from:

```text
What SmartCommit actually does
```

This provides a safety boundary.

```text
Analysis
   |
   v
CommitPlan
   |
   +--> Preview
   |
   +--> Edit
   |
   +--> Validate
   |
   v
Execution
```

A plan should be serializable eventually so that it can be inspected or stored.

---

# 20. Timeline Module

## Responsibility

Generate planned timestamps for commits.

The timeline module must not create commits.

It only produces:

```text
Commit 1 -> Timestamp
Commit 2 -> Timestamp
Commit 3 -> Timestamp
```

Supported MVP modes:

```text
Fixed
Random range
Custom start time
```

Future modes:

```text
Realistic
Complexity-based
AI-assisted
Historical reconstruction
```

---

# 21. Timeline Separation

The timeline engine should remain independent from Git execution.

```text
CommitPlan
    |
    v
TimelineEngine
    |
    v
TimedCommitPlan
    |
    v
Executor
```

This makes the timeline behavior easy to test.

---

# 22. Safety Layer

The safety layer protects the repository before execution.

Its responsibilities include:

```text
Precondition validation
Checkpoint creation
Execution validation
Failure detection
Recovery metadata
Post-operation verification
```

The safety layer must be involved before the executor performs destructive or history-changing operations.

---

# 23. Safety State Machine

The commit operation should conceptually follow:

```text
                 +----------------+
                 |    ANALYZE     |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |    VALIDATE    |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |     PLAN       |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |    PREVIEW     |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |    APPROVAL    |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |   CHECKPOINT   |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |    EXECUTE     |
                 +-------+--------+
                         |
                         v
                 +----------------+
                 |    VERIFY      |
                 +-------+--------+
                         |
                         v
                     COMPLETE
```

Any critical failure should stop execution.

---

# 24. Checkpoint Architecture

Before modifying history, SmartCommit should record the original state.

Conceptually:

```rust
SafetyCheckpoint {
    original_head,
    branch,
    backup_ref,
    created_at,
}
```

The checkpoint must allow SmartCommit to determine where the repository was before the operation.

The implementation should prefer native Git references or another robust Git-native mechanism rather than copying the entire repository.

---

# 25. Execution Layer

## Responsibility

Turn an approved `CommitPlan` into actual Git commits.

The executor receives:

```text
Validated CommitPlan
+
Timeline
+
SafetyCheckpoint
```

and performs the operations.

The executor must not decide how files are grouped.

That decision has already been made.

---

# 26. Critical Git Execution Requirement

SmartCommit must protect unstaged changes.

Example:

```text
File A
  staged changes

File B
  staged changes

File C
  unstaged changes
```

If the plan contains:

```text
Commit 1 -> File A
Commit 2 -> File B
```

File C must remain untouched.

The executor must never use destructive shortcuts such as:

```bash
git reset --hard
```

to simplify implementation.

---

# 27. Index Management

The most sensitive part of commit splitting is controlling the Git index.

The executor must ensure that each commit contains exactly the intended staged changes.

Potential implementation approaches include:

```text
Git index manipulation
Temporary Git index
GIT_INDEX_FILE
libgit2 index operations
Git CLI where appropriate
```

The final implementation should be selected based on:

```text
Correctness
Cross-platform behavior
Safety
Testability
Git compatibility
```

Do not choose an approach merely because it requires fewer lines of code.

---

# 28. Commit Execution Sequence

For:

```text
Commit A
Commit B
Commit C
```

execution should conceptually be:

```text
Create checkpoint
      |
      v
Prepare Commit A
      |
      v
Validate index
      |
      v
Create Commit A
      |
      v
Verify Commit A
      |
      v
Prepare Commit B
      |
      v
Validate index
      |
      v
Create Commit B
      |
      v
Verify Commit B
      |
      v
Prepare Commit C
      |
      v
Validate index
      |
      v
Create Commit C
      |
      v
Verify Commit C
```

If Commit B fails:

```text
Commit A -> completed
Commit B -> failed
Commit C -> not attempted
```

SmartCommit must report this clearly.

---

# 29. Failure Handling

SmartCommit should never hide partial execution.

Example:

```text
Execution failed at commit 2 of 4.

Completed:
1. fix(auth): improve validation

Failed:
2. fix(register): prevent duplicate users

Not executed:
3. docs: update README
4. style: improve dashboard

Recovery checkpoint:
refs/smartcommit/backups/...
```

The user should know exactly what happened.

---

# 30. Recovery Module

The recovery module manages SmartCommit operation metadata.

Potential storage:

```text
.git/smartcommit/
```

Example:

```text
.git/smartcommit/
├── last-operation.toml
└── operations/
```

Do not store source code unnecessarily.

---

# 31. Operation Record

An operation record may contain:

```text
Operation ID
Start time
Original HEAD
Final HEAD
Original branch
Checkpoint reference
Created commits
Operation status
```

Example:

```toml
id = "..."
original_head = "..."
final_head = "..."
branch = "main"
status = "completed"
```

This allows SmartCommit to understand what it did.

---

# 32. Restore Architecture

Restore should not blindly reset the repository.

Before restoration:

```text
Inspect current state
       |
       v
Compare against operation record
       |
       v
Determine whether restoration is safe
       |
       v
Show developer what will happen
       |
       v
Require confirmation
       |
       v
Restore if safe
```

If restoration could destroy unrelated work, SmartCommit should refuse automatic restoration.

---

# 33. CLI Architecture

The CLI should use command-oriented design.

Initial commands:

```text
smartcommit doctor
smartcommit analyze
smartcommit plan
smartcommit commit
smartcommit restore
smartcommit config
```

Potential future commands:

```text
smartcommit search
smartcommit trace
smartcommit history
smartcommit bisect
```

---

# 34. CLI Command Flow

Example:

```bash
smartcommit plan --messages-file commits.txt
```

Flow:

```text
CLI
 |
 v
Parse arguments
 |
 v
Discover repository
 |
 v
Inspect state
 |
 v
Read staged changes
 |
 v
Parse messages
 |
 v
Match changes
 |
 v
Build CommitPlan
 |
 v
Render plan
```

No Git modifications occur.

---

# 35. Commit Command Flow

```bash
smartcommit commit --messages-file commits.txt
```

Flow:

```text
CLI
 |
 v
Discover repository
 |
 v
Inspect state
 |
 v
Analyze changes
 |
 v
Build plan
 |
 v
Generate timeline
 |
 v
Validate
 |
 v
Preview
 |
 v
Developer approval
 |
 v
Checkpoint
 |
 v
Execute
 |
 v
Verify
 |
 v
Report
```

---

# 36. Dry-Run Architecture

Dry-run is an important safety feature.

```bash
smartcommit commit --dry-run
```

Dry-run should execute:

```text
Discovery
Analysis
Matching
Planning
Timeline generation
Validation
Preview generation
```

It must not perform:

```text
Commit creation
Index destruction
History rewriting
Repository reset
```

The same planning code should be used for dry-run and real execution.

---

# 37. Configuration Architecture

Configuration should be loaded in a predictable order:

```text
CLI arguments
      |
      v
Project configuration
      |
      v
Default values
```

Potential project configuration:

```text
.smartcommit.toml
```

Only implemented settings should be accepted.

Unknown configuration should produce a clear error or warning according to the chosen configuration policy.

---

# 38. Error Architecture

Errors should be structured.

Use Rust error types rather than string-only errors throughout the core.

Conceptually:

```rust
SmartCommitError
```

with categories such as:

```text
RepositoryError
StateError
DiffError
PlanningError
ConfigurationError
SafetyError
ExecutionError
RecoveryError
```

The CLI converts these into user-friendly messages.

---

# 39. Error Boundary

The core should provide meaningful structured errors.

Example:

```text
Core:
ExecutionError::CommitFailed { index, reason }
```

The CLI can display:

```text
Commit 2 failed.

Reason:
Git hook rejected the commit.

No further commits were created.
```

The core should not decide terminal formatting.

---

# 40. Logging

Use structured logging where appropriate.

The project should eventually use `tracing`.

Logging levels:

```text
ERROR
WARN
INFO
DEBUG
TRACE
```

Normal CLI output should not be confused with diagnostic logs.

Potential options:

```bash
smartcommit --verbose
smartcommit --quiet
```

Future versions may support:

```bash
smartcommit --debug
```

---

# 41. Security Architecture

Security must be considered before adding AI.

SmartCommit should eventually detect:

```text
API keys
Tokens
Passwords
Private keys
Credential files
Database connection strings
```

However, secret detection does not need to be part of the earliest Git-splitting implementation.

When implemented:

```text
Detect
  |
  v
Redact
  |
  v
Warn/Block
```

Secrets must never be written to logs.

---

# 42. AI Architecture

AI is a future layer, not an MVP dependency.

The architecture should eventually look like:

```text
                 Analysis
                    |
                    v
             AI Abstraction
                    |
        +-----------+-----------+
        |           |           |
        v           v           v
      Local       Cloud       Future
       AI           AI        Provider
```

The core should communicate with an abstraction rather than a specific provider.

---

# 43. AI Must Not Control Git Directly

This is a critical architectural rule.

AI may produce:

```text
Classification
Explanation
Suggested grouping
Suggested message
Confidence
```

AI must not directly execute:

```text
git commit
git reset
git checkout
git clean
```

Instead:

```text
AI
 |
 v
Suggestion
 |
 v
Validation
 |
 v
CommitPlan
 |
 v
Developer Approval
 |
 v
Safety Layer
 |
 v
Executor
```

---

# 44. Future Hunk-Level Architecture

Hunk-level splitting is not part of the first MVP.

When implemented:

```text
File
 |
 +-- Hunk 1 -> Commit A
 |
 +-- Hunk 2 -> Commit B
 |
 +-- Hunk 3 -> Commit A
```

This requires a more sophisticated index/staging architecture.

The current MVP should therefore keep file-level grouping cleanly separated from future hunk-level grouping.

---

# 45. Future Code Understanding

Future versions may use Tree-sitter or equivalent parsing to identify:

```text
Functions
Methods
Classes
Structs
Imports
Modules
Symbols
```

This could improve matching:

```text
Commit:
Fix authentication

Function:
validate_token()

File:
src/auth.rs
```

The architecture should allow this analysis to be added without changing the executor.

---

# 46. Data Flow

The complete MVP data flow is:

```text
Filesystem
    |
    v
Git Repository
    |
    v
Repository State
    |
    v
Staged Changes
    |
    v
Diff Model
    |
    +------------------+
    |                  |
    v                  v
Messages           File Analysis
    |                  |
    +--------+---------+
             |
             v
          Matcher
             |
             v
        Match Scores
             |
             v
        Commit Planner
             |
             v
        Commit Plan
             |
             v
       Timeline Engine
             |
             v
      Timed Commit Plan
             |
             v
          Validator
             |
             v
          Preview
             |
             v
       User Approval
             |
             v
       Safety Checkpoint
             |
             v
          Executor
             |
             v
       Git Repository
             |
             v
         Verification
```

---

# 47. Separation of Concerns

Each component should have one primary responsibility.

| Component | Responsibility |
|---|---|
| Repository | Find and represent repository |
| State | Inspect repository state |
| Changes | Identify staged files |
| Diff | Analyze staged differences |
| Messages | Parse commit messages |
| Matching | Score file/message relationships |
| Planner | Build commit groups |
| Timeline | Generate timestamps |
| Validation | Detect unsafe plans |
| Safety | Protect repository |
| Executor | Create commits |
| Recovery | Track and restore operations |
| CLI | User interaction |

Avoid putting logic into the wrong layer.

---

# 48. Dependency Direction

Dependencies should flow inward.

Preferred:

```text
CLI
 |
 v
Core
 |
 +--> Repository
 +--> Analysis
 +--> Planning
 +--> Safety
 +--> Execution
```

Avoid:

```text
Core
 |
 v
CLI
```

The core must never depend on the CLI.

---

# 49. Testing Architecture

Testing is a first-class architectural concern.

There should be:

```text
Unit tests
Integration tests
End-to-end tests
```

---

# 50. Unit Tests

Unit tests should cover:

```text
Message parsing
Keyword normalization
Matching
Scoring
Timeline generation
Configuration parsing
Validation
```

These tests should not require real Git repositories where unnecessary.

---

# 51. Git Integration Tests

Git operations should use temporary repositories.

Example:

```text
Temporary Directory
       |
       v
git init
       |
       v
Create files
       |
       v
Modify files
       |
       v
git add
       |
       v
Run SmartCommit
       |
       v
Inspect Git history
```

Never use the developer's actual repository for automated destructive tests.

---

# 52. End-to-End Testing

The primary MVP test should simulate:

```text
Developer makes several unrelated changes
             |
             v
Stages everything
             |
             v
Provides multiple messages
             |
             v
SmartCommit analyzes
             |
             v
SmartCommit creates plan
             |
             v
Developer approves
             |
             v
Multiple commits created
             |
             v
History verified
```

---

# 53. Determinism

Where possible, analysis should be deterministic.

Given identical:

```text
Repository state
Diff
Messages
Configuration
```

the matcher and planner should produce the same result.

Randomized timeline behavior should use an injectable/seedable random source for tests.

---

# 54. Performance Principles

The MVP should remain lightweight.

Avoid:

```text
Unnecessary database
Persistent background service
Large framework
Network requirement
AI requirement
Complex caching
```

A normal repository analysis should be fast enough for interactive CLI use.

Future performance optimizations should be driven by profiling rather than speculation.

---

# 55. Cross-Platform Design

SmartCommit should target:

```text
Linux
Windows
macOS
```

Avoid assuming:

```text
Unix-only paths
Bash
GNU-specific commands
Linux-only filesystem behavior
```

Prefer Rust standard library and cross-platform APIs where possible.

Git-specific behavior should be tested on all supported platforms before stable release.

---

# 56. Repository Safety Rules

The following operations are considered sensitive:

```text
git reset
git checkout
git clean
history rewriting
index manipulation
branch movement
```

Sensitive operations must:

1. Be explicit.
2. Be validated.
3. Be tested.
4. Have a recovery strategy.
5. Never silently execute.

---

# 57. No Silent Destructive Behavior

SmartCommit must never silently:

```text
Delete files
Discard changes
Reset working tree
Overwrite unrelated changes
Rewrite history
Force push
```

If an operation could cause data loss, SmartCommit should either:

```text
Require explicit confirmation
```

or:

```text
Refuse to perform it
```

depending on the operation.

---

# 58. Developer Control

AI and automation should remain subordinate to developer decisions.

The developer should be able to:

```text
Accept plan
Reject plan
Edit message
Move file
Remove file
Merge groups
Split groups
Reorder commits
Change timeline
Cancel operation
```

This principle should remain valid even after AI is introduced.

---

# 59. Future TUI

A terminal UI may eventually be added using a dedicated interface layer.

Architecture:

```text
              SmartCommit Core
                     |
            +--------+--------+
            |                 |
            v                 v
           CLI               TUI
```

The TUI must not duplicate planning or Git execution logic.

---

# 60. Future IDE Integration

A future VS Code extension could use SmartCommit through:

```text
CLI
```

or:

```text
Library/API
```

Potential workflow:

```text
VS Code
   |
   v
SmartCommit Core
   |
   v
Commit Plan
```

This is why the core must remain independent from terminal rendering.

---

# 61. Future Semantic History

Later versions may introduce:

```text
smartcommit search
smartcommit trace
smartcommit history
```

These should consume Git history through a dedicated history-analysis layer.

They should not modify the existing commit executor architecture.

---

# 62. Architectural Constraints

The following constraints must remain enforced:

### Constraint 1

The CLI must not contain Git business logic.

### Constraint 2

The core must not depend on terminal output.

### Constraint 3

The planner must not modify Git.

### Constraint 4

The matcher must not create commits.

### Constraint 5

The timeline engine must not modify Git.

### Constraint 6

AI must not directly execute Git commands.

### Constraint 7

The executor must operate only on an approved and validated plan.

### Constraint 8

Destructive operations must have a safety strategy.

### Constraint 9

Tests must not modify real developer repositories.

### Constraint 10

Future functionality must not unnecessarily complicate the MVP.

---

# 63. MVP Architecture Summary

The MVP can be summarized as:

```text
+--------------------------------------------------+
|                     CLI                          |
|                                                  |
| doctor | analyze | plan | commit | restore      |
+-------------------------+------------------------+
                          |
                          v
+--------------------------------------------------+
|                  SMARTCOMMIT CORE                |
|                                                  |
| Repository                                       |
| State                                            |
| Changes                                          |
| Diff                                             |
| Messages                                         |
| Matching                                         |
| Planning                                         |
| Timeline                                         |
| Validation                                       |
| Safety                                           |
| Execution                                        |
| Recovery                                         |
| Configuration                                   |
+-------------------------+------------------------+
                          |
                          v
+--------------------------------------------------+
|                       GIT                        |
|                                                  |
| HEAD | INDEX | WORKING TREE | HISTORY           |
+--------------------------------------------------+
```

---

# 64. Development Strategy

SmartCommit should be implemented vertically and incrementally.

Do not build empty abstractions for every future feature.

Recommended development order:

```text
Workspace
   |
   v
Repository
   |
   v
Repository State
   |
   v
Staged Changes
   |
   v
Diff
   |
   v
Messages
   |
   v
Matching
   |
   v
Planning
   |
   v
Timeline
   |
   v
Safety
   |
   v
Execution
   |
   v
Recovery
   |
   v
CLI Polish
   |
   v
End-to-End Testing
```

Each stage must have tests before the next stage begins.

---

# 65. Definition of Architectural Success

The MVP architecture is successful when:

```text
The CLI can be replaced without rewriting the core.

The matcher can be replaced without rewriting the executor.

The deterministic matcher can later be replaced or enhanced by AI.

The timeline engine can evolve without affecting Git analysis.

The executor can be tested independently from the AI layer.

The safety layer can reject an invalid plan before Git is modified.

The entire planning process can run without modifying the repository.
```

---

# 66. Long-Term Architecture

The eventual system may evolve toward:

```text
                         SmartCommit
                              |
        +---------------------+---------------------+
        |                     |                     |
        v                     v                     v
      CLI                    TUI                  IDE
        |                     |                     |
        +---------------------+---------------------+
                              |
                              v
                       SmartCommit Core
                              |
       +----------+-----------+-----------+----------+
       |          |           |           |          |
       v          v           v           v          v
      Git      Analysis      AI        History    Security
     Engine     Engine      Engine     Engine      Engine
       |          |           |           |          |
       +----------+-----------+-----------+----------+
                              |
                              v
                       Commit Planner
                              |
                              v
                        Safety Layer
                              |
                              v
                         Git Executor
```

The MVP should be considered the foundation of this architecture rather than a separate throwaway prototype.

---

# 67. Final Architectural Principle

SmartCommit should follow this rule throughout development:

> **Analyze first. Plan second. Validate third. Show the developer. Protect the repository. Execute last.**

The architecture exists to enforce that principle.

```text
Analyze
   |
   v
Understand
   |
   v
Plan
   |
   v
Validate
   |
   v
Preview
   |
   v
Approve
   |
   v
Protect
   |
   v
Execute
   |
   v
Verify
```

SmartCommit should never sacrifice repository safety for automation.

---

# 68. Current Implementation Target

The immediate implementation target is:

```text
SmartCommit v0.1

Rust
 |
 +-- Repository detection
 +-- Repository state
 +-- Staged change discovery
 +-- Diff extraction
 +-- Multiline commit messages
 +-- Deterministic file matching
 +-- Commit planning
 +-- Timeline generation
 +-- Safety checkpoint
 +-- Multiple commit execution
 +-- Recovery
 +-- CLI
 +-- Tests
```

Everything else should wait until this foundation is reliable.
---

# Implementation Status Addendum

The sections above are the full architectural specification, authored
before implementation began. This addendum tracks how the actual
workspace maps onto that specification — updated as modules land. For
exact per-module completion status, see `PROJECT_STATE.md`.

## Actual workspace layout (Module 0)

```text
crates/
├── smartcommit-core/   library crate — §7.1
└── smartcommit-cli/    binary crate — §7.2, depends on smartcommit-core
```

This matches §6's target layout. `tests/`, `fixtures/`, and `docs/`
exist as specified; `CONTRIBUTING.md`, `SECURITY.md`, and
`CHANGELOG.md` are not yet created (deferred until Module 15 per the
module build order — no functionality depends on them yet).

## Dependency policy in practice

Per §54 (avoid speculative complexity): dependencies are added only
when a module actually needs them, not preemptively for the whole
architecture.

- `smartcommit-cli` depends on `clap` (derive API) from Module 0
  onward — needed even for `--version`/`--help`, and is the natural
  foundation for the subcommand structure in §33.
- `smartcommit-core` added `thiserror` in Module 1 (for
  `RepositoryError` and friends — §38's "use Rust error types rather
  than string-only errors" applies from the first error type
  onward, not deferred to a later unifying pass).
- `anyhow`, `serde`, and `toml` remain unused as of Module 5.
  `serde`/`toml` are anticipated for Module 13 (Configuration, §37)
  and will be added then, not before.

## §9–§14 module-to-file mapping (as implemented)

| Architecture module (§) | Implementation file |
|---|---|
| §9 Repository | `crates/smartcommit-core/src/repository.rs` |
| §10 Repository State | `crates/smartcommit-core/src/repository_state.rs` |
| §11 Change Discovery | `crates/smartcommit-core/src/staged_changes.rs` |
| §12 Diff | `crates/smartcommit-core/src/diff.rs` |
| §13 Message | `crates/smartcommit-core/src/message_parser.rs` |
| §14–§16 Matching | `crates/smartcommit-core/src/matcher.rs` |
| §17–§19 Commit Planner | `crates/smartcommit-core/src/planner.rs` |
| §20–§21 Timeline | `crates/smartcommit-core/src/timeline.rs` |

Each is a plain Rust module within `smartcommit-core`, per §8 ("These
can initially be Rust modules within one crate. They do not need to
become separate crates.").

## Git integration approach (relevant to §38, §51, §54)

All Git access so far shells out to the system `git` binary (`std::process::Command`)
rather than linking `libgit2`/`git2`. This has been sufficient
through Module 5 (Repository, State, Changes, Diff, Messages) since
none of those need transactional index control. §27 (Index
Management) explicitly leaves the final approach open pending
correctness/safety/testability evaluation — that evaluation is
deferred to Module 10 (Commit Executor), where index manipulation
first becomes safety-critical, rather than decided speculatively now.
