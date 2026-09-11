# Module 7 — Testing Manual: Commit Planner

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-6 tests still pass

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: `51 passed; 0 failed` (5 CLI + 46 core)

## 2. Full plan: two confident groups + one unassigned file
```bash
mkdir -p /tmp/t7 && cd /tmp/t7 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init
mkdir -p src/auth src/dashboard src/shared
echo "fn login() {}" > src/auth/login.rs
echo "fn widgets() {}" > src/dashboard/widgets.rs
echo "fn config() {}" > src/shared/config.rs
git add -A

cat > /tmp/t7-commits.txt <<'EOF'
Fix authentication in login module

Improve dashboard widgets
EOF

smartcommit plan --messages-file /tmp/t7-commits.txt
```
✅ Pass:
- `Commit Plan` header
- `[1] Fix authentication in login module` with `src/auth/login.rs` and a `Confidence:` line
- `[2] Improve dashboard widgets` with `src/dashboard/widgets.rs` and a `Confidence:` line
- `Unassigned:` section listing `src/shared/config.rs`
- exit code 0

## 3. Empty group is reported, not dropped
```bash
cat > /tmp/t7-commits2.txt <<'EOF'
Fix authentication in login module

Add payment validation
EOF

smartcommit plan --messages-file /tmp/t7-commits2.txt
```
✅ Pass: `[2] Add payment validation` prints `(no matching files)` and `Confidence: 0%`, plus a `Warnings:` line mentioning it has no matching files.

## 4. No staged changes
```bash
cd /tmp/t7 && git add -A && git commit --quiet -m "commit everything"
smartcommit plan --messages-file /tmp/t7-commits.txt
echo $?
```
✅ Pass: `No staged changes found.`, exit code 1.

## 5. Empty messages file
```bash
printf '   \n\t \n' > /tmp/t7-empty.txt
smartcommit plan --messages-file /tmp/t7-empty.txt
echo $?
```
✅ Pass: stderr mentions `message input is empty`, exit code 1.

## 6. No Git modification
```bash
cd /tmp/t7
git status --short --branch > /tmp/t7-before.txt
git log --oneline > /tmp/t7-before-log.txt
smartcommit plan --messages-file /tmp/t7-commits.txt > /dev/null
git status --short --branch > /tmp/t7-after.txt
git log --oneline > /tmp/t7-after-log.txt
diff /tmp/t7-before.txt /tmp/t7-after.txt && diff /tmp/t7-before-log.txt /tmp/t7-after-log.txt
```
✅ Pass: both `diff`s produce no output — `plan` never modifies the repository.

## If anything fails
Paste the exact command and output back — especially if the
confidence percentages or grouping differ from what's shown here,
since that points at the matcher's scoring rather than the planner.
