# Module 9 — Testing Manual: Safety Checkpoint Engine

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-8 tests still pass

Like Modules 6 and 8, there's no CLI command for this module yet —
`create_safety_checkpoint` is meant to be called internally by
Module 10 (Commit Executor) right before it starts creating commits,
not run standalone. Verification here is via the test suite plus a
manual check of the underlying Git mechanism.

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `72 passed; 0 failed` (5 CLI + 67 core)

## 2. Manually inspect the checkpoint mechanism
```bash
mkdir -p /tmp/t9 && cd /tmp/t9 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > f.txt && git add f.txt && git commit --quiet -m init

HEAD=$(git rev-parse HEAD)
git update-ref --create-reflog refs/smartcommit/backups/20260901T090000Z "$HEAD" ""
```
✅ Pass: exits 0, no error

```bash
git for-each-ref refs/smartcommit
```
✅ Pass: shows the ref pointing at `$HEAD`

```bash
git branch
```
✅ Pass: shows only your normal branches (e.g. `master`) — the backup
ref must **never** appear here

```bash
# try to create the exact same ref name again
git update-ref --create-reflog refs/smartcommit/backups/20260901T090000Z "$HEAD" ""
echo $?
```
✅ Pass: fails (non-zero exit, `fatal: ... reference already exists`)
— this is the mechanism the collision-handling logic relies on

## 3. What the test suite proves beyond the manual check
- **Backup points to correct HEAD** — the ref resolves to exactly
  the commit that was HEAD at checkpoint time.
- **Branch and snapshot capture** — `branch` matches the actual
  current branch; `staged_files` matches what was actually staged.
- **Detached HEAD** — `branch` is `None` when HEAD isn't on a named
  branch.
- **Backup remains valid after commits** — after the "SmartCommit"
  process goes on to make new commits, the checkpoint ref still
  exists and still points at the *original* pre-operation commit,
  not the new one.
- **Failure doesn't modify the repository** — attempting a
  checkpoint on a brand-new repo with zero commits (HEAD unborn)
  fails cleanly (`CheckpointError::NoCommits`), and the `.git`
  directory's entry count is identical before and after the failed
  attempt.
- **Collision handling** — two checkpoints requested with the exact
  same timestamp get distinct ref names (`...Z` and `...Z-1`), both
  resolving to the same commit.

## If anything fails
Paste the exact test name and output back — for the manual
`update-ref` check specifically, also paste your git version
(`git --version`), since ref-locking error message wording can vary
slightly across git releases.
