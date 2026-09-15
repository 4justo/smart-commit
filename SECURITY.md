# Security Policy

## What SmartCommit does to your repository

SmartCommit is a local tool. It makes no network requests and sends
no data anywhere outside your machine.

### What it reads

- Your Git repository's object database, index, and refs (via
  `git` subprocesses).
- Staged file contents (diffs only — not the full working tree).
- `.smartcommit.toml` at the repository root, if present.

### What it writes

Before creating any commits:

- A backup ref under `refs/smartcommit/backups/<timestamp>`, pointing
  to your current HEAD. This is the recovery point used by
  `smartcommit restore`. It does not appear in `git branch`.

After creating commits:

- `.git/smartcommit/last-operation.toml` — metadata about the most
  recent operation (commit hashes, branch name, timestamps). No source
  code or secrets are stored here.
- `.git/smartcommit/operations/<id>.toml` — a per-operation copy of
  the same record, kept for history.

**The real `.git/index` (your staged changes) is never modified.**
SmartCommit uses a temporary index file (`GIT_INDEX_FILE`) scoped to
each individual commit, then discards it. Your staged state is
identical before and after a `smartcommit commit` run.

### What it does not touch

- Your working tree (unstaged files and edits are never read beyond
  what `git status` reports, and are never modified).
- Any branch other than the one currently checked out.
- Your Git configuration (`~/.gitconfig`, `.git/config`).
- Credentials, SSH keys, GPG keys, or tokens of any kind.

---

## Recovery

If something goes wrong mid-operation, SmartCommit prints a recovery
checkpoint ref:

```
Recovery checkpoint: refs/smartcommit/backups/2026-09-15T...
```

To undo the operation manually:

```bash
git update-ref refs/heads/<your-branch> <checkpoint-ref>
```

Or use the built-in restore command, which performs this check and
shows you exactly what will change before asking for confirmation:

```bash
smartcommit restore
```

`smartcommit restore` refuses automatically if anything has changed
since the operation (a new commit on the branch, etc.) and prints
the exact manual command you would need instead.

---

## Reporting a vulnerability

SmartCommit is a local developer tool with no network surface and no
privilege escalation. If you discover a way it could corrupt a
repository, destroy data, or behave unsafely in a reproducible
scenario, please open an issue on the project repository describing:

1. The exact commands you ran
2. The repository state before the commands
3. The observed outcome
4. The expected outcome

There is no dedicated security email address at v0.1.0. Issues are
the correct channel.
