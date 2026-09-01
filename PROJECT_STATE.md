# PROJECT_STATE

## Current version
v0.1-dev (pre-release, Git Splitter milestone)

## Current milestone
v0.1 — Git Splitter (MVP, deterministic only, no AI)

## Completed features
- Project scaffolded with `cargo new smartcommit --bin`
- Git Engine: `is_git_repository(path)` — detects whether a given
  path is inside a Git working tree, via `git rev-parse
  --is-inside-work-tree` shelled out to the system `git` binary
  - Returns `Ok(true)` / `Ok(false)` / `Err(GitEngineError)`
    (the Err case is reserved for "git binary itself could not be
    run", not "not a repo")
  - Unit tests: real temp git repo → true; plain temp dir → false
    (both passing as of this writing, verified with `cargo test`)
  - CLI wired up in `main.rs`: prints result, sets exit code
    (0 = repo detected, 1 = not a repo or git unavailable),
    manually verified against both a real repo and `/tmp`

## Current feature
Nothing in progress — step 1 of the v0.1 MVP checklist is done and
tested. Next unstarted step is staged-changes detection.

## Known bugs
None identified yet (surface area is still tiny).

## Architecture decisions
- **ADR-001 (informal, not yet in docs/adr/):** MVP shells out to
  the system `git` CLI via `std::process::Command` instead of
  linking `git2` (libgit2 bindings).
  - Reason: repo detection is trivial via CLI; `git2` adds real
    dependency weight (links libgit2, larger build, requires libgit2
    dev headers) for no benefit at this stage.
  - **Revisit in v0.2** when we need staged-diff retrieval and
    hunk-level parsing — reliably parsing `git diff` text output
    gets fragile fast, and `git2` gives structured diff/hunk objects
    instead. Decide then whether to switch the whole Git Engine to
    `git2` or keep CLI-shelling for simple ops and add `git2` only
    for diff parsing.

## Pending tasks (v0.1 MVP checklist, in order)
1. ✅ Detect Git repository
2. ⬜ Detect staged changes (`git diff --cached --name-status` or
   equivalent)
3. ⬜ Read staged diff content
4. ⬜ Read multiline commit messages (from stdin/file/flag — format TBD)
5. ⬜ Parse messages into individual logical messages
6. ⬜ Identify modified files (from step 2/3 output)
7. ⬜ Match messages to files (basic deterministic logic — e.g. order-based
   or keyword-based, NOT semantic/AI yet)
8. ⬜ Group files
9. ⬜ Display proposed commits (preview output)
10. ⬜ Developer approval step (interactive confirm)
11. ⬜ Create separate commits
12. ⬜ Configurable time differences between commits
13. ⬜ Recovery checkpoint (backup branch) before modifying history

## Important configuration
None yet — no config file schema decided (deferred per section 38
of AI_DEVELOPMENT_CONTEXT.md, "do not finalize until implementation
begins").

## Last successful test
`cargo test` — 2 passed, 0 failed (git_engine::tests::detects_true_inside_a_real_git_repo,
git_engine::tests::detects_false_in_a_plain_directory)

## Next task
Step 2 of the MVP checklist: staged-changes detection — a function
in `git_engine.rs` that lists staged files (path + status: added /
modified / deleted / renamed) via `git diff --cached --name-status`,
with unit tests using a temp repo with staged changes in each
status category.
