# Module 13 — Testing Manual: Configuration

## Prerequisites
- You're in the `smartcommit/` repo root
- Modules 0-12 tests still pass

## 1. Build & test
```bash
cargo build --workspace
cargo test --workspace
```
✅ Pass: all tests pass, 0 failed.

## 2. Default config (no file present)
```bash
mkdir -p /tmp/t13 && cd /tmp/t13 && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > README.md && git add README.md && git commit --quiet -m init
echo "fn login() {}" > src_auth.rs
git add -A

echo "Fix authentication" > /tmp/t13-commits.txt
smartcommit plan --messages-file /tmp/t13-commits.txt
```
✅ Pass: tool runs with default values. No error about a missing
`.smartcommit.toml`. Confidence threshold, gap, and confirmation
behaviour match documented defaults.

## 3. Valid config file
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[matching]
minimum_confidence = 0.70

[timeline]
mode = "fixed"
gap_minutes = 10

[commit]
require_confirmation = true
EOF

smartcommit plan --messages-file /tmp/t13-commits.txt
```
✅ Pass: tool loads the file, applies its values, and produces the
same output as the default run (values match the defaults, so
behaviour is identical). No parse error.

## 4. Confidence threshold is respected
```bash
# Use a very high threshold to force a file into Unassigned
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[matching]
minimum_confidence = 0.99
EOF

mkdir -p src/shared
echo "fn helper() {}" > src/shared/utils.rs
git add src/shared/utils.rs

cat > /tmp/t13-commits2.txt <<'EOF'
Fix authentication

Update documentation
EOF

smartcommit plan --messages-file /tmp/t13-commits2.txt
```
✅ Pass: files that would have been assigned at the default threshold
now appear under `Unassigned:`, because the raised threshold demotes
low-confidence matches.

## 5. Timeline gap from config
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[timeline]
mode = "fixed"
gap_minutes = 30
EOF

echo "fn login() {}" > auth.rs
echo "fn widgets() {}" > dashboard.rs
git add auth.rs dashboard.rs

cat > /tmp/t13-commits3.txt <<'EOF'
Fix authentication

Improve dashboard
EOF

smartcommit plan --messages-file /tmp/t13-commits3.txt
```
✅ Pass: `Time:` lines are 30 minutes apart.

## 6. CLI flag overrides config
```bash
# Config says gap_minutes = 30, CLI overrides to 5
smartcommit plan --messages-file /tmp/t13-commits3.txt --interval-minutes 5
```
✅ Pass: `Time:` lines are 5 minutes apart, not 30. The CLI value
wins over the config value.

## 7. require_confirmation = false skips the prompt
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[commit]
require_confirmation = false
EOF

git add auth.rs dashboard.rs
smartcommit commit --messages-file /tmp/t13-commits3.txt
echo $?
```
✅ Pass: commits are created without presenting `Create these
commits? [y/N]`, exit 0. Equivalent to passing `--yes` from the CLI.

## 8. require_confirmation config is overridden by --yes
Re-stage with `require_confirmation = true` in the config:
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[commit]
require_confirmation = true
EOF

git add auth.rs dashboard.rs
smartcommit commit --messages-file /tmp/t13-commits3.txt --yes
echo $?
```
✅ Pass: commits are created without a prompt (explicit `--yes` wins),
exit 0.

## 9. Invalid TOML syntax
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[matching
minimum_confidence = 0.70
EOF

smartcommit plan --messages-file /tmp/t13-commits.txt
echo $?
```
✅ Pass: clear error message mentioning `.smartcommit.toml` and a
TOML parse failure (line number helpful but not required), exit 1.
No panic.

## 10. Invalid value range
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[matching]
minimum_confidence = 1.5
EOF

smartcommit plan --messages-file /tmp/t13-commits.txt
echo $?
```
✅ Pass: readable error explaining that `minimum_confidence` must be
between 0.0 and 1.0, exit 1. No panic.

```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[timeline]
mode = "fixed"
gap_minutes = -5
EOF

smartcommit plan --messages-file /tmp/t13-commits.txt
echo $?
```
✅ Pass: readable error explaining that `gap_minutes` must be
positive, exit 1. No panic.

## 11. Unknown keys are handled gracefully
```bash
cat > /tmp/t13/.smartcommit.toml <<'EOF'
[matching]
minimum_confidence = 0.70
typo_key = "oops"
EOF

smartcommit plan --messages-file /tmp/t13-commits.txt
echo $?
```
✅ Pass: either a clear warning about the unknown key or a clean
error. The tool must not silently ignore it and must not panic.
Document the actual observed behaviour here.

## 12. Config in project root is found automatically
```bash
# Confirm the file is picked up from the working directory, not from
# the SmartCommit binary's directory or $HOME.
mkdir -p /tmp/t13-sub && cd /tmp/t13-sub && git init --quiet
git config user.email a@b.com && git config user.name t
echo x > f.txt && git add f.txt && git commit --quiet -m init

# Place a config that sets a recognisable gap
cat > .smartcommit.toml <<'EOF'
[timeline]
mode = "fixed"
gap_minutes = 42
EOF

echo "fn a() {}" > a.rs && echo "fn b() {}" > b.rs && git add -A

cat > /tmp/t13-sub-commits.txt <<'EOF'
Change A

Change B
EOF

smartcommit plan --messages-file /tmp/t13-sub-commits.txt
```
✅ Pass: `Time:` lines are 42 minutes apart, confirming the config
was discovered from the project root.

## If anything fails
Paste the exact command and full output. For validation errors,
confirm the exit code with `echo $?` immediately after the failing
command. The most important thing to verify is that bad config always
produces a readable message and never a Rust panic/backtrace.
