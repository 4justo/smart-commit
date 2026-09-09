# Module 5 — Testing Manual: Multiline Commit Message Parser

## Prerequisites
- You are in the `smartcommit/` repo root.
- The workspace builds and the `smartcommit` binary is available under `target/debug/`.

## 1. Build the workspace
```bash
cargo build --workspace
```
Pass: compiles cleanly.

## 2. Run all tests
```bash
cargo test --workspace
```
Pass: all workspace tests succeed.

## 3. Parse a messages file
```bash
cat > /tmp/commits.txt <<'EOF'
Fix authentication

Improve dashboard UI

Update README
EOF

./target/debug/smartcommit plan --messages-file /tmp/commits.txt
```
Pass:
- exits 0
- prints `Parsed 3 commit message(s):`
- prints each message in order

## 4. Reject empty input
```bash
printf '   \n\t \n' > /tmp/empty-commits.txt
./target/debug/smartcommit plan --messages-file /tmp/empty-commits.txt
```
Pass:
- exits 1
- prints an error mentioning `message input is empty`

## 5. Line-ending and trimming check
```bash
printf '  Fix authentication  \r\n\r\n  Update README  \r\n' > /tmp/crlf-commits.txt
./target/debug/smartcommit plan --messages-file /tmp/crlf-commits.txt
```
Pass:
- exits 0
- trims surrounding whitespace
- handles CRLF input

## If anything fails
Record the exact command and output before changing code.
