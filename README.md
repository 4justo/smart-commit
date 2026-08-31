# SmartCommit

> **AI-powered semantic Git commit management and history reconstruction for developers.**

SmartCommit is a professional developer tool built in **Rust** that analyzes Git changes, understands their logical relationships, and helps developers transform large, messy sets of changes into clean, meaningful, traceable commits.

Instead of creating one massive commit containing multiple unrelated changes, SmartCommit analyzes files, diffs, code, commit messages, and eventually AI-generated semantic insights to create a structured commit history.

```text
Messy Changes
      |
      v
+-----------------+
|   SmartCommit   |
|                 |
| Git Analysis    |
| Diff Analysis   |
| AI Reasoning    |
| Planning        |
+--------+--------+
         |
         v
Clean Git History
```

---

# The Problem

Developers often work on multiple features, fixes, experiments, and documentation changes before committing.

For example:

```text
Modified files:

src/auth/login.rs
src/auth/jwt.rs
src/dashboard/mod.rs
src/dashboard/styles.css
README.md
tests/auth_tests.rs
```

A developer might create one large commit:

```bash
git commit -m "Fixed authentication, improved dashboard, updated README"
```

This creates problems:

* Difficult bug tracing
* Poor Git history
* Harder code reviews
* Unrelated changes mixed together
* Difficult use of `git bisect`
* Difficult `git blame` analysis
* Harder rollback of individual features
* Reduced project maintainability

SmartCommit helps transform this into:

```text
feat(auth): improve JWT authentication

style(dashboard): improve dashboard layout

docs: update project documentation

test(auth): add authentication tests
```

Each commit contains only the changes logically related to that purpose.

---

# Project Vision

SmartCommit aims to become an intelligent Git history management system that can:

* Understand source-code changes
* Detect logical changes
* Split large changes into multiple commits
* Match commit messages to files and code changes
* Support AI-powered semantic analysis
* Support hunk-level commit splitting
* Respect code dependencies
* Generate clean commit messages
* Create safe and recoverable Git operations
* Improve bug traceability
* Help developers maintain professional Git history

> **The goal is not to replace the developer. The goal is to make Git history cleaner, safer, and easier to understand.**

---

# Built With Rust

SmartCommit is built primarily in **Rust**.

Rust provides:

* High performance
* Memory safety
* Cross-platform support
* Excellent CLI tooling
* Strong concurrency support
* Native binaries
* Reliable filesystem operations
* A strong ecosystem for developer tooling

The project is designed as a professional CLI-first developer tool.

---

# Core Features

## 1. Semantic Commit Splitting

SmartCommit analyzes a collection of changes and separates them into logical commits.

Example:

```text
Changes:

Authentication
├── JWT validation
├── Login redirect
└── Session handling

Dashboard
├── Layout
└── Responsive design

Documentation
└── README
```

SmartCommit can propose:

```text
Commit 1
fix(auth): improve JWT validation

Commit 2
fix(auth): prevent login redirect loop

Commit 3
style(dashboard): improve responsive layout

Commit 4
docs: update README
```

---

## 2. Multiline Commit Message Support

Developers can provide multiple intended commit messages.

Example:

```text
Fix authentication

Improve dashboard UI

Update documentation

Add payment validation
```

SmartCommit parses these messages and attempts to align each message with the appropriate changes.

---

## 3. Automatic Logical Change Detection

Users should not always need to manually provide commit messages.

SmartCommit can eventually detect logical groups automatically.

Example:

```text
Analyzing 27 files...

Detected:

1. Authentication changes
2. Registration validation
3. Dashboard improvements
4. Payment processing
5. Documentation
```

The developer can review and modify every suggestion.

---

# AI-Powered Semantic Analysis

SmartCommit is designed with an AI provider architecture.

AI can be used to understand:

* Code changes
* Commit intent
* File relationships
* Diff hunks
* Functions and methods
* Classes and modules
* Dependencies
* Natural-language commit messages

Example:

```text
Commit Message:

"Fix login redirect"

Changed files:

login.rs        96%
auth.rs         88%
dashboard.rs    12%
README.md        3%
```

The tool uses these relationships to build a proposed commit plan.

---

# File-Level Analysis

SmartCommit analyzes:

* Added files
* Modified files
* Deleted files
* Renamed files
* File paths
* File extensions
* Directory structure
* Related files

Example:

```text
src/auth/
├── jwt.rs
├── login.rs
└── session.rs
```

These files are likely related and can be analyzed together.

---

# Diff Analysis

SmartCommit analyzes Git diffs to understand what actually changed.

It can inspect:

* Added lines
* Deleted lines
* Modified code
* Diff hunks
* Changed functions
* Changed classes
* Changed methods
* Imports
* Symbols

Example:

```text
Changed file:

auth_service.rs

Modified functions:

validate_token()
refresh_token()
authenticate_user()
```

This provides significantly better grouping than simply matching filenames.

---

# Hunk-Level Commit Splitting

One file can contain multiple unrelated changes.

Example:

```text
UserService.rs

Hunk 1
→ Login validation

Hunk 2
→ Registration logic

Hunk 3
→ Logging improvements
```

SmartCommit will eventually support:

```text
UserService.rs

Login changes
      |
      v
Commit 1

Registration changes
      |
      v
Commit 2

Logging changes
      |
      v
Commit 3
```

This is one of the project's most important advanced capabilities.

---

# Dependency-Aware Commit Ordering

SmartCommit should understand logical dependencies.

Example:

```text
Database Model
       |
       v
Backend Service
       |
       v
API Endpoint
       |
       v
Frontend Integration
```

The commit history should respect this order whenever possible.

SmartCommit can propose:

```text
Database
      |
      v
Backend
      |
      v
API
      |
      v
Frontend
```

---

# Confidence Scoring

AI decisions should never be treated as unexplained facts.

SmartCommit assigns confidence levels.

```text
jwt.rs

Authentication Commit

Confidence: 97%
```

| Score     | Meaning              |
| --------- | -------------------- |
| 95–100%   | Very High Confidence |
| 85–94%    | High Confidence      |
| 70–84%    | Moderate Confidence  |
| Below 70% | Requires Review      |

Confidence thresholds should be configurable.

---

# Explainable AI Decisions

Developers should be able to ask:

```text
Why was this file assigned to this commit?
```

Example:

```text
auth_service.rs

Assigned to:

fix(auth): prevent invalid session

Confidence: 93%

Reasons:

- Modified validate_session()
- Added JWT validation
- References authentication middleware
- Strong semantic similarity
- Located in authentication module
```

SmartCommit should never become an unexplained black box.

---

# AI Commit Message Generation

If the developer does not provide commit messages, SmartCommit can propose them.

```bash
smartcommit analyze
```

Example output:

```text
Suggested commits:

1. feat(auth): add refresh token support

2. fix(register): prevent duplicate email registration

3. style(dashboard): improve responsive layout

4. docs: update installation instructions
```

The developer can:

* Accept
* Edit
* Merge
* Split
* Delete
* Reorder

all suggestions before any commit is created.

---

# Conventional Commit Support

SmartCommit supports Conventional Commit styles.

```text
feat:
fix:
docs:
style:
refactor:
test:
chore:
perf:
build:
ci:
```

Example:

```text
feat(auth): add refresh token support

fix(api): prevent duplicate registration

test(auth): add JWT validation tests

docs: update installation guide
```

Commit styles should be configurable.

---

# Commit Timeline Engine

SmartCommit supports configurable commit timelines.

Instead of multiple commits appearing with the exact same timestamp:

```text
14:30
14:30
14:30
14:30
```

SmartCommit can plan a timeline:

```text
14:05  Authentication
14:19  Registration
14:37  Dashboard
15:02  Payments
15:18  Documentation
```

## Timeline Modes

### Fixed

Every commit receives a fixed interval.

### Random

Intervals are selected from a configured range.

### Realistic

Intervals can be estimated using factors such as:

* Number of files
* Number of changed lines
* Code complexity
* Number of functions changed
* Dependency relationships

### Custom

Developers manually configure timestamps.

### Reconstruction / Replay

SmartCommit can create a synthetic reconstruction plan for logically organizing commits within a specified time window.

> Reconstructed timestamps should always be clearly identified as synthetic rather than presented as factual historical records.

---

# Safety First

SmartCommit must prioritize user code and repository safety.

Before major Git operations:

```text
Validate
    |
    v
Preview
    |
    v
Developer Approval
    |
    v
Safety Checkpoint
    |
    v
Execute
    |
    v
Verify
```

---

# Recovery Checkpoints

Before modifying Git history, SmartCommit should create a recovery point.

Example:

```text
smartcommit-backup-2026-08-31-2015
```

Output:

```text
Safety checkpoint created

Branch:
smartcommit-backup-2026-08-31-2015

HEAD:
a91f4c2
```

If something fails, the developer should be able to restore the previous state.

---

# Interactive Preview

SmartCommit should always provide a clear preview.

```text
SMARTCOMMIT PLAN

Detected: 6 logical commits


[1] feat(auth): add JWT validation

Files:
  src/auth/jwt.rs
  src/auth/session.rs

Confidence: 96%


[2] fix(register): prevent duplicate users

Files:
  src/register/service.rs
  tests/register_tests.rs

Confidence: 91%


[3] docs: update README

Files:
  README.md

Confidence: 99%


[A] Accept
[E] Edit
[S] Split
[M] Merge
[R] Re-analyze
[Q] Quit
```

---

# Full Developer Control

The developer should always be able to:

* Move files
* Move hunks
* Rename commits
* Edit commit messages
* Merge commit groups
* Split commit groups
* Reorder commits
* Reject AI suggestions
* Adjust timestamps
* Adjust confidence thresholds
* Cancel operations

AI suggestions must never remove developer control.

---

# Test Awareness

SmartCommit should recognize relationships between implementation and tests.

Example:

```text
src/auth/service.rs
tests/auth/service_tests.rs
```

SmartCommit can recommend:

```text
Keep implementation and relevant tests
inside the same logical commit.
```

---

# Generated File Detection

SmartCommit should detect common generated files:

```text
node_modules/
dist/
build/
target/
bin/
obj/
__pycache__/
*.generated.*
```

Generated files should be handled according to project configuration.

---

# Secret Detection

Before committing, SmartCommit should scan for possible secrets.

Examples include:

* API keys
* Access tokens
* Passwords
* Private keys
* Cloud credentials
* Database URLs

Example:

```text
SECURITY WARNING

Potential secret detected:

.env

Commit creation blocked.

Review the detected file before continuing.
```

Secret values must never be printed in logs.

---

# Repository Health Checks

SmartCommit should inspect the repository before performing operations.

It should detect:

* Merge conflicts
* Merge in progress
* Rebase in progress
* Cherry-pick in progress
* Detached HEAD
* Unstaged changes
* Staged changes
* Partially staged files
* Untracked files
* Repository locks

Example:

```bash
smartcommit doctor
```

Output:

```text
Git installed
Git repository detected
Repository state valid
Configuration valid
No merge conflicts
AI provider available
Ready
```

---

# Commit Quality Analysis

SmartCommit should evaluate commit quality.

Example:

```text
Commit Quality Report

One logical purpose
Related files
Message matches changes
Tests detected
No secrets detected

Score: 94/100
```

Warnings:

```text
Possible unrelated changes detected.

Recommendation:

Split this commit into:

1. Authentication changes
2. Dashboard changes
```

---

# Semantic Git History Search

SmartCommit should eventually support semantic history queries.

```bash
smartcommit search "authentication changes"
```

Or:

```bash
smartcommit search "when was payment validation introduced?"
```

The system should search Git history based on meaning rather than only exact keywords.

---

# Bug Traceability

One of SmartCommit's long-term goals is improved debugging.

```bash
smartcommit trace "login redirect bug"
```

Potential output:

```text
Possible related commit:

fix(auth): change login redirect handling

Confidence: 94%

Files:

src/auth/login.rs
src/auth/session.rs

Relevant function:

handle_login()
```

---

# Regression Analysis

SmartCommit may eventually integrate with Git history analysis.

```bash
smartcommit bisect
```

The system could help explain likely regressions:

```text
Likely regression:

refactor(auth): simplify session validation

Changed function:

validate_session()

Potential impact:

Login authentication flow
```

---

# AI Provider Architecture

SmartCommit should not depend on a single AI provider.

The architecture should support:

```text
                AI Provider
                     |
       +-------------+-------------+
       |             |             |
       v             v             v

    Local AI      Cloud AI      Future AI
```

Possible providers include:

## Local

* Ollama
* LM Studio
* Other local inference engines

## Cloud

* Configurable AI providers through a provider abstraction layer

---

# Privacy Modes

Developers should control how source code is analyzed.

Possible modes:

```text
local-only
cloud-enabled
hybrid
```

Additional controls:

```text
redact-secrets
minimum-context
disable-code-upload
```

SmartCommit should never silently send source code to a cloud AI provider.

---

# Configuration

SmartCommit should support project-level configuration.

Example:

```toml
[commits]
style = "conventional"

[analysis]
confidence_threshold = 0.85

[timeline]
mode = "realistic"
min_gap_minutes = 5
max_gap_minutes = 25

[ai]
provider = "local"

[security]
block_secrets = true
```

Configuration should remain extensible.

---

# CLI

SmartCommit is CLI-first.

Potential commands:

```bash
smartcommit
smartcommit analyze
smartcommit preview
smartcommit commit
smartcommit restore
smartcommit config
smartcommit doctor
smartcommit search
smartcommit trace
```

---

# Analyze

```bash
smartcommit analyze
```

Analyze changes without creating commits.

---

# Preview

```bash
smartcommit preview
```

Display the proposed commit plan.

---

# Commit

```bash
smartcommit commit
```

Create approved commits.

---

# Dry Run

```bash
smartcommit --dry-run
```

Example:

```text
Dry Run Enabled

No Git changes will be made.

Detected:

5 logical commits

Proposed commit history:

1. feat(auth): improve authentication
2. fix(register): prevent duplicates
3. feat(payment): validate transactions
4. style(ui): improve dashboard
5. docs: update README
```

---

# Restore

```bash
smartcommit restore
```

Restore a previous checkpoint when supported by the operation.

---

# Doctor

```bash
smartcommit doctor
```

Check whether SmartCommit and the repository are ready.

---

# Planned Architecture

```text
                         SmartCommit
                              |
             +----------------+----------------+
             |                |                |
             v                v                v

        Git Engine       Analysis Engine     AI Engine
             |                |                |
             v                v                v

         Repository        Files            Local AI
         Index             Diffs            Cloud AI
         Commits           Hunks            Embeddings
         Branches          Symbols          LLM
             |                |                |
             +----------------+----------------+
                              |
                              v

                       Commit Planner
                              |
                 +------------+------------+
                 |            |            |
                 v            v            v

             Dependencies   Timeline      Validation
                              |
                              v

                         Safety Layer
                              |
                              v

                        CLI / TUI / API
```

---

# Planned Project Structure

```text
smartcommit/

├── crates/
│
│   ├── smartcommit-core/
│   │   ├── git/
│   │   ├── diff/
│   │   ├── staging/
│   │   ├── commits/
│   │   ├── planner/
│   │   ├── timeline/
│   │   ├── safety/
│   │   └── validation/
│   │
│   ├── smartcommit-analysis/
│   │   ├── files/
│   │   ├── hunks/
│   │   ├── symbols/
│   │   └── dependencies/
│   │
│   ├── smartcommit-ai/
│   │   ├── providers/
│   │   ├── embeddings/
│   │   ├── prompts/
│   │   └── matching/
│   │
│   └── smartcommit-cli/
│       ├── commands/
│       ├── output/
│       └── interactive/
│
├── tests/
├── fixtures/
│
├── docs/
│   └── adr/
│
├── README.md
├── ARCHITECTURE.md
├── PROJECT_STATE.md
├── AI_DEVELOPMENT_CONTEXT.md
├── CONTRIBUTING.md
├── SECURITY.md
├── CHANGELOG.md
└── Cargo.toml
```

The project should grow into this structure gradually. Do not create unnecessary modules before they are needed.

---

# Testing Strategy

SmartCommit should have:

* Unit tests
* Integration tests
* End-to-end tests
* Temporary Git repository tests
* AI matching tests
* CLI tests
* Recovery tests
* Failure tests

Git operations must be tested in isolated temporary repositories.

Never run destructive automated tests against a real developer repository.

---

# Important Edge Cases

SmartCommit must eventually handle:

* Empty repositories
* No staged changes
* Untracked files
* Deleted files
* Renamed files
* Binary files
* Large files
* Large repositories
* Partially staged files
* Same file belonging to multiple commits
* Merge conflicts
* Rebase operations
* Cherry-pick operations
* Detached HEAD
* Git hooks
* AI unavailable
* Ambiguous AI results
* Invalid configuration
* Interrupted operations

---

# Safety Guarantees

SmartCommit should aim to provide the following guarantees:

* Never silently destroy code
* Never silently rewrite history
* Never expose detected secrets
* Never automatically trust low-confidence AI decisions
* Always provide previews for significant operations
* Create recovery mechanisms before risky operations
* Stop safely when an operation fails

---

# Development Roadmap

## v0.1 — Core Git Splitter

* [ ] Rust project setup
* [ ] Repository detection
* [ ] Git status inspection
* [ ] Staged change detection
* [ ] Diff extraction
* [ ] Multiline message parsing
* [ ] Basic file grouping
* [ ] Commit plan generation
* [ ] Preview
* [ ] Developer confirmation
* [ ] Multiple commit creation
* [ ] Configurable timeline
* [ ] Safety checkpoint
* [ ] Basic recovery
* [ ] Integration tests

---

## v0.2 — Semantic Analysis

* [ ] File analysis
* [ ] Directory context
* [ ] Keyword extraction
* [ ] Symbol analysis
* [ ] Semantic matching
* [ ] Confidence scoring
* [ ] Explainability
* [ ] Better grouping

---

## v0.3 — Hunk Intelligence

* [ ] Hunk parsing
* [ ] Hunk classification
* [ ] Multi-commit file support
* [ ] Hunk-level preview
* [ ] Safe partial staging
* [ ] Dependency-aware ordering

---

## v0.4 — Interactive TUI

* [ ] Interactive commit preview
* [ ] Move files
* [ ] Move hunks
* [ ] Merge commits
* [ ] Split commits
* [ ] Reorder commits
* [ ] Edit messages
* [ ] Edit timestamps

---

## v0.5 — AI Engine

* [ ] AI provider abstraction
* [ ] Local AI support
* [ ] Cloud AI support
* [ ] AI commit generation
* [ ] Semantic code understanding
* [ ] AI explanations
* [ ] AI fallback

---

## v0.6 — Timeline Intelligence

* [ ] Timeline modes
* [ ] Complexity estimation
* [ ] Custom schedules
* [ ] Reconstruction mode
* [ ] Synthetic timeline labeling

---

## v0.7 — Security and Quality

* [ ] Secret detection
* [ ] Generated file detection
* [ ] Test awareness
* [ ] Commit quality scoring
* [ ] Repository health analysis

---

## v0.8 — History Intelligence

* [ ] Semantic Git history search
* [ ] Bug tracing
* [ ] Regression analysis
* [ ] Intelligent Git bisect assistance

---

## v0.9 — Integrations

* [ ] Git hooks
* [ ] GitHub integration
* [ ] GitLab integration
* [ ] CI/CD
* [ ] VS Code extension

---

## v1.0 — Stable Release

Before v1.0:

* [ ] Extensive testing
* [ ] Cross-platform support
* [ ] Stable CLI
* [ ] Reliable recovery
* [ ] Security review
* [ ] Performance testing
* [ ] Complete documentation
* [ ] Installation guides
* [ ] Release binaries

---

# Planned Rust Technology Stack

| Component       | Technology                      |
| --------------- | ------------------------------- |
| Language        | Rust                            |
| CLI             | `clap`                          |
| Git Integration | `git2` and/or Git CLI           |
| Terminal UI     | `ratatui`                       |
| Async Runtime   | `tokio` where required          |
| Serialization   | `serde`                         |
| Configuration   | TOML                            |
| Code Parsing    | Tree-sitter                     |
| Logging         | `tracing`                       |
| Error Handling  | `thiserror` / `anyhow`          |
| Testing         | Rust unit and integration tests |

Dependencies should only be added when they are actually required.

---

# Contributing

SmartCommit is designed to become an open-source developer tool.

Contributors should:

1. Understand the issue before changing code.
2. Keep changes focused.
3. Write tests.
4. Avoid unnecessary dependencies.
5. Preserve Git safety.
6. Document important architectural decisions.
7. Never introduce destructive behavior without recovery mechanisms.

---

# Development Philosophy

SmartCommit follows one fundamental principle:

> **Correctness and safety are more important than automation.**

AI can make mistakes.

Git operations can be destructive.

Therefore:

```text
AI Suggestion
      |
      v
Validation
      |
      v
Preview
      |
      v
Developer Review
      |
      v
Safety Checkpoint
      |
      v
Execution
      |
      v
Verification
```

The developer remains in control.

---

# Long-Term Goal

SmartCommit aims to evolve from a commit-splitting CLI into an intelligent Git development assistant capable of helping developers understand:

* What changed
* Why it changed
* Which changes belong together
* When a feature was introduced
* Which commit may have introduced a bug
* How code evolved
* Which commits are logically related

---

# Project Principles

```text
Safe

Transparent

Explainable

Developer-controlled

AI-assisted

Cross-platform

Fast

Maintainable

Privacy-aware

Open-source friendly
```

---

# Project Status

> **Early Development**

SmartCommit is currently in the architecture and foundation stage.

The first development milestone is:

## v0.1 — Core Git Splitter

The focus is to build a reliable Rust foundation before introducing advanced AI features.

---

# License

License to be decided.

Potential options:

* MIT
* Apache-2.0
* Dual MIT / Apache-2.0

---

# Final Vision

```text
A messy development session
            |
            v
      +-------------+
      | SmartCommit |
      +------+------+
             |
             v
     Understand changes
             |
             v
      Detect intentions
             |
             v
      Group logically
             |
             v
      Validate safely
             |
             v
       Developer review
             |
             v
    Clean Git history
```

> **SmartCommit doesn't replace Git.**
>
> **It helps developers use Git history more intelligently.**
>
> **Built with Rust. Designed for developers. Powered by intelligence.**
