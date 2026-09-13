# Module 11 — Testing Manual: Preview and Confirmation CLI

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-10 tests still pass

This is the module where the tool actually works end to end — worth
testing by hand, not just trusting the suite.

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `87 passed; 0 failed` (9 CLI + 78 core)

## 2. Full plan preview
```bash
mkdir -p /tmp/t11 && cd /tmp/t11 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init
mkdir -p src/auth src/dashboard src/shared
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
echo "fn config() {}" > src/shared/config.rs
git add -A

cat > /tmp/t11-commits.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

smartcommit plan --messages-file /tmp/t11-commits.txt
```
✅ Pass: `SMARTCOMMIT PLAN` header, `3 staged files`, `2 proposed
commits`, each group shows files/Confidence/Time, `Unassigned:` lists
`src/shared/config.rs`, a `Warnings:` line explains why. Exit 0.

## 3. Timeline flags
```bash
smartcommit plan --messages-file /tmp/t11-commits.txt --start "2026-09-01T09:00:00+03:00" --interval-minutes 15
```
✅ Pass: `Time: 09:00` then `Time: 09:15`.

```bash
smartcommit plan --messages-file /tmp/t11-commits.txt --random-min-minutes 5 --random-max-minutes 20
```
✅ Pass: succeeds, two `Time:` lines with some gap between them.

```bash
smartcommit plan --messages-file /tmp/t11-commits.txt --random-min-minutes 5
echo $?
```
✅ Pass: fails with "must be given together", exit 1.

## 4. Dry run makes no changes
```bash
git status --short --branch > /tmp/before.txt
git log --oneline > /tmp/before-log.txt

smartcommit commit --messages-file /tmp/t11-commits.txt --dry-run
echo $?

git status --short --branch > /tmp/after.txt
git log --oneline > /tmp/after-log.txt
diff /tmp/before.txt /tmp/after.txt && diff /tmp/before-log.txt /tmp/after-log.txt
```
✅ Pass: prints the full preview plus `Dry run: no changes were
made.`, exit 0, both `diff`s produce no output.

## 5. Real commit with --yes
```bash
smartcommit commit --messages-file /tmp/t11-commits.txt --yes
echo $?
git log --oneline
git status --short
```
✅ Pass: `All commits created successfully.` with two `N. message
(oid)` lines, exit 0. `git log` shows the two new commits.
`git status` still shows `src/shared/config.rs` staged — the
unassigned file was never committed.

## 6. Confirmation defaults to No
Re-stage the same files for a second round, then:
```bash
smartcommit commit --messages-file /tmp/t11-commits.txt
# press Enter with no input
echo $?
```
✅ Pass: prints `Create these commits? [y/N]`, and pressing Enter
alone prints `Cancelled.`, exit code **2** specifically (not 1).

## 7. Blocked repository is refused
```bash
mkdir -p /tmp/t11b && cd /tmp/t11b && git init --quiet
git config user.email a@b.com && git config user.name t
echo base > conflict.txt && git add conflict.txt && git commit --quiet -m base
git checkout --quiet -b feature
echo feature > conflict.txt && git commit --quiet -am feature
git checkout --quiet -
echo main > conflict.txt && git commit --quiet -am main
git merge feature   # this will conflict — expected

echo "Resolve conflict" > /tmp/t11b-commits.txt
smartcommit plan --messages-file /tmp/t11b-commits.txt
echo $?
```
✅ Pass: refuses with "not in a safe state" and "a merge is in
progress", exit 1. `git merge --abort` to clean up afterward.

## 8. Partial failure with a real hook
```bash
mkdir -p /tmp/t11c && cd /tmp/t11c && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init
echo "fn login() {}" > login.rs
echo "fn widgets() {}" > widgets.rs
git add -A

mkdir -p .git/hooks
cat > .git/hooks/pre-commit <<'HOOK'
#!/bin/sh
if git diff --cached --name-only | grep -q widgets.rs; then
  echo "REJECTED: widgets.rs blocked by policy" >&2
  exit 1
fi
exit 0
HOOK
chmod +x .git/hooks/pre-commit

cat > /tmp/t11c-commits.txt <<'EOF'
Fix login

Fix widgets
EOF

smartcommit commit --messages-file /tmp/t11c-commits.txt --yes
echo $?
```
✅ Pass: `Execution failed at commit 2 of 2.`, a `Completed:` section
showing commit 1, a `Failed:` section with the hook's own rejection
message, a `Recovery checkpoint:` line with a real ref. Exit code
**3** specifically (not 1). Confirm the ref is real:
```bash
git for-each-ref refs/smartcommit
```

## If anything fails
Paste the exact command and output back — for exit codes especially,
run `echo $?` immediately after the command (some shells reset it).
