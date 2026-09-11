# Module 0 — Testing Manual: Rust Workspace Foundation

## Prerequisites
- You're inside the `smartcommit/` repo root (where the workspace `Cargo.toml` lives)
- `.git` is intact and connected to your GitHub remote

## 1. Build the workspace
```bash
cargo build --workspace
```
✅ Pass: compiles with no errors, ends in "Finished dev [unoptimized + debuginfo] target(s)..."

## 2. Run all tests
```bash
cargo test --workspace
```
✅ Pass: `3 passed; 0 failed` total, across:
- `smartcommit-core`: 1 test (`placeholder_types_are_constructible`)
- `smartcommit-cli`: 2 tests (`version_flag_prints_version_and_exits_zero`, `help_flag_prints_help_and_exits_zero`)

## 3. Manual CLI checks
```bash
./target/debug/smartcommit --version
```
✅ Pass: prints `smartcommit 0.1.0`

```bash
./target/debug/smartcommit --help
```
✅ Pass: prints usage text, exit code 0
```bash
echo $?    # should print 0
```

```bash
./target/debug/smartcommit
```
✅ Pass: prints help text (no subcommands to run yet), exit code 2
```bash
echo $?    # should print 2
```

## 4. Sanity check: no Git behavior exists yet
```bash
./target/debug/smartcommit doctor
```
✅ Pass (for Module 0): this should **fail** with an "unrecognized subcommand" error — `doctor` isn't implemented until Module 1. If it succeeds or does anything Git-related, that's a scope leak worth flagging.

## If anything fails
Paste the exact command and output back into the chat — don't try to patch it yourself first, since a passing local fix might mask something the module logic needs to actually handle.
