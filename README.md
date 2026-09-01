# SmartCommit

SmartCommit is a professional Git commit-planning and splitting
tool, written in Rust. It lets a developer stage a batch of related
changes, describe the several logical commits hidden inside that
batch in plain text, and have SmartCommit propose — and, once
approved, create — those commits safely.

**Status: pre-alpha (Module 0 of the v0.1 MVP build plan).** Almost
nothing described below is implemented yet; see "Currently
implemented" for the honest, current state.

## Currently implemented

- `smartcommit --version`
- `smartcommit --help`

That's it. No Git behavior exists yet — repository detection,
staged-change analysis, commit planning, and commit creation are
all still ahead. See `PROJECT_STATE.md` for exact progress against
the module build order, and `ARCHITECTURE.md` for the workspace
layout.

## Planned workflow (not yet implemented)

```bash
git add .
smartcommit plan --messages-file commits.txt
# review the proposed grouping and timeline
smartcommit commit --messages-file commits.txt
```

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
├── Cargo.toml
├── README.md
├── ARCHITECTURE.md
└── PROJECT_STATE.md
```

`smartcommit-core` must remain usable without AI or any external
LLM, and independent of the CLI.
