# Module 2+3 — Testing Manual

## Build & test
cargo build --workspace
cargo test --workspace
✅ 21 passed, 0 failed

## doctor (Module 2 — now full state)
mkdir -p /tmp/t && cd /tmp/t && git init --quiet
smartcommit doctor
✅ "Repository: valid" / branch / HEAD / staged/unstaged/untracked / "State: warning" (nothing staged yet)

echo x > f.txt && git add f.txt
smartcommit doctor
✅ "State: ready"

## analyze (Module 3)
smartcommit analyze
✅ "Staged changes: 1" then "Added     f.txt"

git commit --quiet -m init
smartcommit analyze
✅ "No staged changes found." + exit 1

echo y >> f.txt   # unstaged only
smartcommit analyze
✅ still "No staged changes found." — unstaged must NOT appear

echo new > untracked.txt   # untracked only
smartcommit analyze
✅ still "No staged changes found." — untracked must NOT appear

## Rename detection
mv f.txt renamed.txt && git add -A
smartcommit analyze
✅ "Renamed   f.txt -> renamed.txt"
