# SmartCommit

SmartCommit is a professional Git commit-planning and splitting tool,
written in Rust. It lets a developer stage a batch of related changes,
describe the several logical commits hidden inside that batch in plain
text, and have SmartCommit propose — and, once approved, create — those
commits safely.

**NEW: AI-Powered Features**
SmartCommit now includes AI capabilities for automatic commit message generation
and semantic file-to-commit matching using multiple AI providers (Claude, OpenAI, local LLMs).

**Status: MVP complete through Module 14 (end-to-end test suite) with AI integration (Module 15+).**
The full pipeline — planning, configuration, committing, restore, and
automated testing — is implemented and passing.

## Currently implemented

```bash
smc doctor        # repository detection + full state (branch, HEAD, conflicts, staged/unstaged/untracked)
smc analyze       # staged files with diff summaries (--verbose for full hunks)
smc plan          # full preview: proposed commits, confidence, timestamps, unassigned files, warnings
smc commit        # plan, confirm ([y/N], or --yes), and create real commits (or --dry-run)
smc restore       # undo the most recent SmartCommit operation, if it's safe to do so automatically
smc ai-generate   # generate commit messages using AI from staged changes
```

All commands above are real and tested end to end.

## Workflow

```bash
git add .

# preview only — makes no Git modification
smc plan --messages-file commits.txt

# review the proposed grouping and timeline, then create the commits
smc commit --messages-file commits.txt
# or skip the confirmation prompt:
smc commit --messages-file commits.txt --yes
# or preview without committing anything:
smc commit --messages-file commits.txt --dry-run
```

If you need to undo it afterward:

```bash
smc restore
# or, non-interactively:
smc restore --yes
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

### AI Features

SmartCommit now includes AI-powered capabilities for automatic commit message generation:

```bash
# Generate commit messages using AI from staged changes
smc ai-generate

# Generate multiple atomic commits instead of one comprehensive message
smc ai-generate --split

# Specify AI provider (claude, openai, local)
smc ai-generate --provider claude
```

**AI Configuration:**

Set environment variables to configure AI providers:

```bash
# For Claude (Anthropic)
export ANTHROPIC_API_KEY="your-api-key"

# For OpenAI
export OPENAI_API_KEY="your-api-key"

# For local LLM (Ollama)
export OLLAMA_BASE_URL="http://localhost:11434"

# Choose provider (optional, defaults to claude)
export SMC_AI_PROVIDER="claude|openai|local"
```

**AI Features:**
- **Automatic message generation**: AI analyzes your code changes and writes conventional commit messages
- **Semantic file matching**: AI understands the actual meaning of changes and groups files intelligently
- **Multiple providers**: Support for Claude, OpenAI, and local LLMs (Ollama)
- **Fallback mechanism**: Automatically falls back to keyword matching if AI fails
- **Privacy-focused**: Option to use local LLMs for sensitive codebases

**Note:** AI features are optional. SmartCommit works perfectly without AI using the original keyword-based matching system.

### Configuration

Place a `.smc.toml` at your repository root to tune default
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

For AI configuration, you can also add an `[ai]` section to `.smc.toml`:

```toml
[ai]
provider = "claude"  # claude, openai, or local
enable_message_generation = true
enable_semantic_matching = true
fallback_to_keywords = true

[ai.model]
claude = "claude-3-sonnet-20240229"
openai = "gpt-4-turbo-preview"
local = "llama2"

[ai.generation]
max_tokens = 1024
temperature = 0.7
```

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
│   └── smartcommit-cli/    # CLI binary (installs as `smc`) + integration tests
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

The `smartcommit-core` library remains usable without AI or any external LLM,
and is independent of the CLI.
