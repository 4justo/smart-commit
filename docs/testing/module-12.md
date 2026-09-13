# Module 12 — Testing Manual: Restore and Recovery

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-11 tests still pass

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `103 passed; 0 failed` (12 CLI + 91 core)

## 2. Successful restore
```bash
mkdir -p /tmp/t12 && cd /tmp/t12 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init
mkdir -p src/auth src/dashboard
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
git add -A

cat > /tmp/t12-commits.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

ORIGINAL_HEAD=$(git rev-parse HEAD)
smartcommit commit --messages-file /tmp/t12-commits.txt --yes
git log --oneline
```
✅ Pass: two new commits on top of `init`.

```bash
smartcommit restore --yes
echo $?
```
✅ Pass: `SMARTCOMMIT RESTORE`, `Safety: OK`, `Restored to <original head>`,
exit 0.

```bash
git rev-parse HEAD    # should equal $ORIGINAL_HEAD
git status --short    # should show both files staged again (A src/auth/login.rs, A src/dashboard/widgets.rs)
```
✅ Pass: HEAD back to the pre-commit state, both files staged again —
restore fully undid the operation with no extra logic needed, because
the real index was never touched throughout.

## 3. Operation metadata
```bash
cat .git/smartcommit/last-operation.toml
ls .git/smartcommit/operations/
```
✅ Pass: TOML with `id`, `start_time`, `original_head`, `final_head`,
`branch`, `checkpoint_ref`, `created_commits`, `status = "completed"`.
The `operations/` directory has a file with the same `id`.

## 4. Restore after multiple commits
Repeat step 2 with three staged files instead of two (three commit
messages). Confirm `git log --oneline` shows 3 new commits before
restore, and exactly 1 commit (`init`) after.

## 5. Changed working tree after the operation doesn't block restore
```bash
# after a fresh commit --yes run
echo "unstaged edit" >> src/auth/login.rs
echo "new" > untracked.txt

smartcommit restore --yes
```
✅ Pass: still restores successfully (`Safety: OK`, working tree
changes are informational only). Confirm the unstaged edit and the
untracked file are still there afterward, untouched.

## 6. Unsafe restore refusal
```bash
# after a fresh commit --yes run
echo "unrelated work" > unrelated.txt
git add unrelated.txt
git commit --quiet -m "an unrelated later commit"

smartcommit restore --yes
echo $?
```
✅ Pass: `Safety: BLOCKED`, a reason mentioning the HEAD mismatch, an
exact `git update-ref refs/heads/<branch> <original_head>` manual
command printed, exit 1. Confirm `git log --oneline` is completely
unaffected by the refused attempt.

## 7. No operation to restore
```bash
mkdir -p /tmp/t12-fresh && cd /tmp/t12-fresh && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > f.txt && git add f.txt && git commit --quiet -m init
smartcommit restore --yes
echo $?
```
✅ Pass: `No SmartCommit operation found to restore.`, exit 1.

## If anything fails
Paste the exact command and output back. If a restore appears to
succeed but the staged files don't come back correctly, that's the
most important thing to flag precisely — it would mean the "index is
never touched" guarantee from Module 10 broke somewhere, which this
whole module depends on.
