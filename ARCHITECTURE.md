# Architecture

## Workspace layout

SmartCommit is a two-crate Cargo workspace:

```text
crates/
├── smartcommit-core/   library crate
└── smartcommit-cli/    binary crate, depends on smartcommit-core
```

### Why a workspace, and why split core from CLI

- **`smartcommit-core` must work without AI or an external LLM**,
  and independent of any particular front end. Keeping it a
  separate library crate makes that a compile-time guarantee rather
  than a convention someone can accidentally violate — the CLI
  crate depends on core, never the other way around.
- It also means a future front end (TUI, editor plugin, etc.) could
  reuse `smartcommit-core` directly instead of shelling out to the
  CLI binary.
- Splitting early (Module 0) avoids a disruptive later refactor once
  real logic exists — Module 0 explicitly scaffolds this shape
  before any Git behavior is written.

### `smartcommit-core`

The Git engine and all planning/matching/timeline logic. No
CLI-specific concerns (argument parsing, terminal output formatting,
`std::process::exit` codes) belong here.

### `smartcommit-cli`

Argument parsing (via `clap`) and presentation. Delegates all actual
work to `smartcommit-core`. Currently (Module 0) this crate does
nothing beyond parsing `--version`/`--help` — no subcommands exist
yet.

## Dependency policy

Dependencies are added only when a module actually needs them, not
speculatively. As of Module 0:

- `smartcommit-cli` depends on `clap` (derive API) for argument
  parsing — needed even for the Module 0 `--version`/`--help`
  surface, and it's the natural foundation for the subcommands
  (`doctor`, `analyze`, `plan`, `commit`, `restore`) added in later
  modules.
- `smartcommit-core` has **no dependencies yet**. `thiserror`,
  `anyhow`, `serde`, and `toml` are anticipated (error types land in
  Module 1; configuration serialization lands in Module 13) but are
  deliberately not added until the module that needs them.

## Data flow (target shape, not yet built)

```text
CLI args
   │
   ▼
smartcommit-cli (parses, dispatches)
   │
   ▼
smartcommit-core
   ├── Repository detection & state         (Modules 1-2)
   ├── Staged change discovery & diffing    (Modules 3-4)
   ├── Message parsing                      (Module 5)
   ├── Deterministic file matching          (Module 6)
   ├── Commit planning                      (Module 7)
   ├── Timeline generation                  (Module 8)
   ├── Safety checkpoint                    (Module 9)
   └── Commit execution                     (Module 10)
   │
   ▼
smartcommit-cli (preview, confirmation, output formatting)
```

This document is updated as each module lands; see `PROJECT_STATE.md`
for exact current status.
