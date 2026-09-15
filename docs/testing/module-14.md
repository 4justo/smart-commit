# Module 14 — Testing Manual: End-to-End MVP Test Suite

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-13 tests still pass
- `smartcommit` binary is on your `$PATH` (or use `cargo run --bin smartcommit --`)

## 1. Build & verify the full suite
```bash
cargo build --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
✅ Pass: `135 passed; 0 failed` (12 cli_invocation + 17 end_to_end + 106 core),
`fmt` and `clippy` both clean.

---

## 2. Scenario 1 — Main scenario (three files → three commits)
```bash
mkdir -p /tmp/t14 && cd /tmp/t14 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src
echo "fn authenticate() {}" > src/auth.rs
echo "fn render_dashboard() {}" > src/dashboard.rs
echo "updated docs" > README.md
git add -A

cat > /tmp/t14-commits.txt <<'EOF'
Fix authentication

Improve dashboard

Update documentation
EOF

smartcommit commit --messages-file /tmp/t14-commits.txt --yes
git log --oneline --reverse
```
✅ Pass: `init` + three new commits in order. Each commit contains
only its own file:
```bash
git diff-tree --no-commit-id -r --name-only HEAD^^   # src/auth.rs only
git diff-tree --no-commit-id -r --name-only HEAD^    # src/dashboard.rs only
git diff-tree --no-commit-id -r --name-only HEAD     # README.md only
```

---

## 3. Scenario 2 — No repository
```bash
mkdir -p /tmp/t14-norepo && cd /tmp/t14-norepo
smartcommit plan --messages-file /tmp/t14-commits.txt
echo $?
```
✅ Pass: non-zero exit, a readable error mentioning "repository" or
"git". No panic.

---

## 4. Scenario 3 — No staged changes
```bash
mkdir -p /tmp/t14-clean && cd /tmp/t14-clean && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > f.txt && git add f.txt && git commit --quiet -m init

smartcommit commit --messages-file /tmp/t14-commits.txt --yes
echo $?
```
✅ Pass: `No staged changes found.`, exit 1.

---

## 5. Scenario 4 — One message, one commit
```bash
mkdir -p /tmp/t14-one && cd /tmp/t14-one && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() {}" > src/auth/login.rs
git add -A

echo "Fix authentication" > /tmp/t14-one.txt
smartcommit commit --messages-file /tmp/t14-one.txt --yes
git log --oneline
```
✅ Pass: `init` + one new "Fix authentication" commit.

---

## 6. Scenario 5 — Multiple messages, multiple commits
```bash
mkdir -p /tmp/t14-multi && cd /tmp/t14-multi && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

echo "fn auth() {}" > src_auth.rs
echo "fn pay() {}" > src_payment.rs
echo "# Guide" > guide.md
git add -A

cat > /tmp/t14-multi.txt <<'EOF'
Fix authentication

Add payment validation

Update documentation
EOF

smartcommit commit --messages-file /tmp/t14-multi.txt --yes
git log --oneline --reverse
```
✅ Pass: three commits in order after `init`.

---

## 7. Scenario 6 — Ambiguous file left unassigned
```bash
mkdir -p /tmp/t14-ambig && cd /tmp/t14-ambig && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth src/dashboard src/shared
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
echo "fn util() {}" > src/shared/utils.rs
git add -A

cat > /tmp/t14-ambig.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

smartcommit plan --messages-file /tmp/t14-ambig.txt
```
✅ Pass: `SMARTCOMMIT PLAN` shows `Unassigned:` containing
`src/shared/utils.rs`.

```bash
smartcommit commit --messages-file /tmp/t14-ambig.txt --yes
git diff --cached --name-only
```
✅ Pass: `src/shared/utils.rs` is still staged — never committed.

---

## 8. Scenario 7 — Unassigned file never committed
```bash
mkdir -p /tmp/t14-unassigned && cd /tmp/t14-unassigned && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() {}" > src/auth/login.rs
echo "fn random() {}" > random_unrelated.rs
git add -A

echo "Fix authentication" > /tmp/t14-unassigned.txt
smartcommit commit --messages-file /tmp/t14-unassigned.txt --yes

git diff --cached --name-only   # must contain random_unrelated.rs
git log --all --name-only --format= --diff-filter=A   # must NOT contain random_unrelated.rs
```
✅ Pass: `random_unrelated.rs` is staged but never appears in any
commit's added-file list.

---

## 9. Scenario 8 — Added file
```bash
mkdir -p /tmp/t14-added && cd /tmp/t14-added && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/payment
echo "fn validate() {}" > src/payment/validate.rs
git add -A

echo "Add payment validation" > /tmp/t14-added.txt
smartcommit commit --messages-file /tmp/t14-added.txt --yes
git diff-tree --no-commit-id -r --name-only HEAD
```
✅ Pass: `src/payment/validate.rs` appears in the commit.

---

## 10. Scenario 9 — Deleted file
```bash
mkdir -p /tmp/t14-deleted && cd /tmp/t14-deleted && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

echo "fn old() {}" > src_legacy.rs
git add src_legacy.rs && git commit --quiet -m "add legacy"
git rm --quiet src_legacy.rs

echo "Remove legacy module" > /tmp/t14-deleted.txt
smartcommit commit --messages-file /tmp/t14-deleted.txt --yes
git log --oneline
ls src_legacy.rs 2>&1   # should say "No such file"
```
✅ Pass: commit created, file gone from disk.

---

## 11. Scenario 10 — Unstaged changes preserved end to end
```bash
mkdir -p /tmp/t14-unstaged && cd /tmp/t14-unstaged && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() { /* v1 */ }" > src/auth/login.rs
git add -A && git commit --quiet -m "add login"

# Stage v2
echo "fn login() { /* v2 staged */ }" > src/auth/login.rs
git add src/auth/login.rs

# Leave v3 as an unstaged edit on top of v2
echo "fn login() { /* v3 unstaged */ }" > src/auth/login.rs

# Add an unrelated unstaged file
echo "work in progress" > scratch.txt

echo "Fix authentication" > /tmp/t14-unstaged.txt
smartcommit commit --messages-file /tmp/t14-unstaged.txt --yes

cat src/auth/login.rs     # must contain v3 unstaged
cat scratch.txt           # must still say "work in progress"
git show HEAD:src/auth/login.rs   # must contain v2 staged
```
✅ Pass: working-tree edit and scratch file survive intact; commit
contains only the staged (v2) version.

---

## 12. Scenario 11 — Timeline timestamps
```bash
mkdir -p /tmp/t14-timeline && cd /tmp/t14-timeline && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth src/dashboard
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
git add -A

cat > /tmp/t14-timeline.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

smartcommit commit \
  --messages-file /tmp/t14-timeline.txt \
  --start "2026-01-01T09:00:00+00:00" \
  --interval-minutes 15 \
  --yes

git log -1 --format="%aI" HEAD^   # 2026-01-01T09:00:00...
git log -1 --format="%aI" HEAD    # 2026-01-01T09:15:00...
```
✅ Pass: first commit author date is 09:00, second is 09:15 — exactly
15 minutes apart.

---

## 13. Scenario 12 — Dry run makes no Git modification
```bash
cd /tmp/t14   # reuse the repo from Scenario 1, or init a fresh one

git status --short --branch > /tmp/before-status.txt
git log --oneline > /tmp/before-log.txt

smartcommit commit --messages-file /tmp/t14-commits.txt --dry-run
echo $?

git status --short --branch > /tmp/after-status.txt
git log --oneline > /tmp/after-log.txt
diff /tmp/before-status.txt /tmp/after-status.txt
diff /tmp/before-log.txt /tmp/after-log.txt
```
✅ Pass: output ends with `Dry run: no changes were made.`, exit 0,
both diffs produce no output.

---

## 14. Scenario 13 — User cancellation exits with code 2
```bash
mkdir -p /tmp/t14-cancel && cd /tmp/t14-cancel && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() {}" > src/auth/login.rs
git add -A

ORIGINAL_HEAD=$(git rev-parse HEAD)

echo "" | smartcommit commit --messages-file /tmp/t14-one.txt
echo $?   # must be 2

git rev-parse HEAD   # must equal $ORIGINAL_HEAD
```
✅ Pass: `Cancelled.` printed, exit code exactly **2** (not 1),
no new commits.

---

## 15. Scenario 14 — Recovery checkpoint created before commits
```bash
mkdir -p /tmp/t14-ckpt && cd /tmp/t14-ckpt && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() {}" > src/auth/login.rs
git add -A

ORIGINAL_HEAD=$(git rev-parse HEAD)

echo "Fix authentication" > /tmp/t14-ckpt.txt
smartcommit commit --messages-file /tmp/t14-ckpt.txt --yes

git for-each-ref refs/smartcommit/backups/
git branch --list   # must NOT contain smartcommit
```
✅ Pass: at least one ref under `refs/smartcommit/backups/` resolves
to `$ORIGINAL_HEAD`. It does not appear in `git branch`.

```bash
git rev-parse $(git for-each-ref --format='%(refname)' refs/smartcommit/backups/ | head -1)
# must print $ORIGINAL_HEAD
```

---

## 16. Scenario 15 — Restore reverts the operation
```bash
mkdir -p /tmp/t14-restore && cd /tmp/t14-restore && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth src/dashboard
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
git add -A

ORIGINAL_HEAD=$(git rev-parse HEAD)

cat > /tmp/t14-restore.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

smartcommit commit --messages-file /tmp/t14-restore.txt --yes
git log --oneline    # 3 commits

smartcommit restore --yes
echo $?              # 0

git rev-parse HEAD   # must equal $ORIGINAL_HEAD
git diff --cached --name-only   # must list both files staged again
```
✅ Pass: `SMARTCOMMIT RESTORE`, `Safety: OK`, `Restored to <original
HEAD>`, exit 0. HEAD is back, both files are staged.

---

## 17. Scenario 16 — Binary file does not crash the tool
```bash
mkdir -p /tmp/t14-binary && cd /tmp/t14-binary && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth assets
echo "fn login() {}" > src/auth/login.rs
printf '\x89PNG\r\n\x1a\n\x00\x00' > assets/logo.png
git add -A

echo "Fix authentication" > /tmp/t14-binary.txt
smartcommit plan --messages-file /tmp/t14-binary.txt
echo $?
smartcommit commit --messages-file /tmp/t14-binary.txt --yes
```
✅ Pass: neither `plan` nor `commit` panics (`thread 'main' panicked`
must not appear in any output). `plan` exits 0. `commit` succeeds or
fails cleanly — the important thing is no Rust backtrace.

---

## 18. Scenario 17 — Invalid config rejected before any Git work
```bash
mkdir -p /tmp/t14-cfg && cd /tmp/t14-cfg && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init

mkdir -p src/auth
echo "fn login() {}" > src/auth/login.rs
git add -A
ORIGINAL_HEAD=$(git rev-parse HEAD)

# Case A — broken TOML syntax
printf '[matching\nminimum_confidence = 0.70\n' > .smartcommit.toml
smartcommit commit --messages-file /tmp/t14-binary.txt --yes
echo $?                  # must be 1
git rev-parse HEAD       # must equal $ORIGINAL_HEAD (no commits created)
```
✅ Pass: error message contains `.smartcommit.toml`, no panic.

```bash
# Case B — valid TOML, out-of-range value
printf '[matching]\nminimum_confidence = 1.5\n' > .smartcommit.toml
smartcommit commit --messages-file /tmp/t14-binary.txt --yes
echo $?                  # must be 1
git rev-parse HEAD       # must still equal $ORIGINAL_HEAD
```
✅ Pass: validation error, no panic, no commits.

---

## If anything fails

Run `cargo test --workspace` first — all 17 `end_to_end` tests must
pass before any manual step is trusted. If a manual step diverges from
the automated test result, the automated test is the ground truth; the
manual reproduction environment may differ (git version, PATH, locale).

The most critical property to verify manually is Scenario 10 (unstaged
changes preserved): if the working-tree edit is destroyed, that breaks
the `GIT_INDEX_FILE` isolation guarantee from Module 10, and everything
downstream of that is suspect.
