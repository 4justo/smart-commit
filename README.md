# Smart Commit CLI

Smart Commit is a command-line assistant that turns a large working tree into a clean, reviewable Git history. It analyses changed files and their diffs, proposes logical groups of related edits, and suggests a branch and commit message for each group. The developer remains in control: every suggestion can be reviewed, changed, skipped, or applied.

## Goals

- Analyse staged and unstaged Git changes, including file paths and diff context.
- Split a broad set of edits into focused, independent commits instead of placing unrelated work under one message.
- Suggest clear conventional-style commit messages and a suitable branch name for the work.
- Let users choose a time interval between generated commits.
- Let users review and edit the date and time of every commit before it is created.
- Create commits only after explicit user confirmation.

## Intended workflow

1. Run Smart Commit from a Git repository after making changes.
2. The CLI inspects the working tree and proposes a branch, for example `feat/commit-planning`.
3. It presents a commit plan: each proposed commit contains its files or hunks, a message, and a scheduled timestamp.
4. Review the plan. Move files or hunks between commits, rename messages, add or remove commits, and change any timestamps.
5. Confirm the plan. Smart Commit stages only the selected changes and creates the commits in order.

Example plan:

```text
Suggested branch: feat/commit-planning

1. feat(cli): add change-analysis command
   files: src/analyse.ts, src/commands/analyse.ts
   date: 2026-08-07 09:00 +03:00

2. test(cli): cover grouped commit suggestions
   files: tests/analyse.test.ts
   date: 2026-08-07 09:15 +03:00

3. docs: explain commit planning workflow
   files: README.md
   date: 2026-08-07 09:30 +03:00
```

## Commit planning rules

The CLI should favour small, coherent commits that can be understood and reverted independently. It may use file location, diff content, imports, and naming to identify related changes, but suggestions are never treated as final.

- Keep implementation, tests, documentation, configuration, and generated files separate when that makes the history clearer.
- Group related edits across multiple files when they represent one change.
- Support file-level grouping first, then hunk-level selection for files containing unrelated edits.
- Detect renames, additions, modifications, and deletions.
- Do not automatically commit secrets, untracked generated output, or ignored files without an explicit opt-in.
- Never rewrite existing history unless a future command explicitly asks for it.

## Time and date controls

When a plan has more than one commit, the user can set a default gap such as `15m`, `1h`, or `1d`. Smart Commit calculates successive timestamps from a chosen starting time, and the user can override any individual value.

Example commands (proposed interface):

```bash
smartcommit plan --interval 15m
smartcommit plan --start "2026-08-07 09:00 +03:00" --interval 1h
smartcommit edit 2 --date "2026-08-07 11:30 +03:00"
smartcommit apply
```

When applying a plan, Smart Commit must set both `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE` so the selected timestamp is faithfully recorded. It should show the resolved timezone and ask for confirmation before writing commits.

## Suggested commands

```text
smartcommit plan [--branch <name>] [--start <datetime>] [--interval <duration>]
smartcommit show
smartcommit edit <commit-number> [--message <message>] [--date <datetime>]
smartcommit move <file-or-hunk> --to <commit-number>
smartcommit apply
smartcommit abort
smartcommit undo
```

`plan` must be safe and read-only. `apply` is the only command that changes the index, creates a branch, or writes commits.

## Interactive plan review

Before creating anything, Smart Commit should provide an interactive review screen as well as non-interactive command options. A developer can accept a suggestion, edit it, split one group into several commits, merge groups, remove a group, or move a file or hunk between groups.

The review should show the proposed branch, message, included paths or hunks, checks to run, and timestamp for every commit. No suggested grouping is applied until the developer explicitly confirms it.

## Commit conventions and team rules

Smart Commit should support Conventional Commits by default, while allowing each team to define its own message format. Rules may include allowed types and scopes, required issue references, maximum subject length, and preferred branch prefixes.

For example, a team could require `feat(api): add commit-plan endpoint` or `fix: resolve timestamp parsing #123`. The CLI should validate messages before applying the plan and explain any failed rule clearly.

## AI, privacy, and fallback behaviour

AI-assisted grouping and message suggestions should be optional and configurable. The CLI should support a chosen provider or a local model, and clearly state what diff content will be sent outside the machine before doing so.

- Provide a privacy mode that keeps analysis local.
- Allow users to exclude paths or redact sensitive content from analysis.
- Never expose API keys in output, plans, logs, or committed files.
- Offer a deterministic non-AI fallback based on Git status, paths, file types, and diff structure.
- Record whether a plan was AI-assisted without storing sensitive diff content by default.

## Quality checks

Projects can configure checks such as tests, linting, formatting, type checking, and secret scanning. Smart Commit should run the relevant checks before each commit or before the final apply step, then display the result in the plan.

Failed checks must prevent application by default, with an explicit, auditable override option for teams that allow it. Checks should run only against the appropriate staged content where the project tooling supports that workflow.

## Saved plans and recovery

A proposed plan should be saveable so that it can be reviewed by a teammate, committed later, or resumed after interruption. Saved plans must include the base commit, selected files or hunks, messages, timestamps, configuration used, and check results. On resume, the CLI should verify that the working tree and base commit still match; otherwise it should require a review instead of silently applying stale selections.

`smartcommit undo` should safely reverse the most recently applied Smart Commit plan when possible, with a clear preview of what it will change. If a failure occurs during application, the CLI must stop, preserve recoverable state, and give exact recovery steps rather than continuing with a partially understood index.

## Git awareness and integration

Before planning or applying, Smart Commit should detect and explain relevant Git state:

- current branch, upstream, ahead/behind status, and default branch;
- partially staged files, merge conflicts, rebases, cherry-picks, and detached HEAD state;
- untracked, ignored, generated, and potentially sensitive files;
- linked worktrees and whether another worktree may be using the same branch.

It should refuse unsafe actions during unresolved conflicts or an in-progress Git operation. Future integrations may create pull requests, link issues, generate changelog entries, and publish plan summaries to CI, but none should run without explicit configuration and confirmation.

## Configuration

Repository settings should live in `.smartcommit.yml`; personal defaults may live in a user-level configuration file. Repository configuration is versioned with the project and should never contain secrets.

```yaml
branchPrefixes:
  feature: feat/
  bugfix: fix/
commitStyle: conventional
timezone: Africa/Nairobi
defaultInterval: 15m
ignoredPaths:
  - dist/**
  - '*.lock'
requiredChecks:
  - npm test
  - npm run lint
messageRules:
  maxSubjectLength: 72
  requireIssueReference: false
privacy:
  mode: local
```

Command-line options take precedence over repository settings. Smart Commit should show the effective configuration when requested, and validate configuration before applying a plan.

## Examples

### Feature across code, tests, and documentation

For a new command that analyses Git changes, Smart Commit may propose one implementation commit containing the command and analysis service, one test commit containing its direct coverage, and one documentation commit. The developer can instead combine code and direct tests if that is the team’s convention.

### Refactor with no behaviour change

For a refactor, group only the moved or simplified production code with the tests updated solely because of that change. Keep opportunistic formatting and unrelated cleanup in separate commits so the refactor remains easy to review.

### Bug fix with a regression test

For a bug fix, prefer a commit that contains the regression test and the minimal production fix, followed by an optional documentation or changelog commit. The plan should explain why related changes belong together.

## Delivery roadmap

### MVP

- Read-only change analysis and a previewable commit plan.
- Branch and commit-message suggestions.
- Interactive editing of commit groups, messages, and timestamps.
- Safe file-level and hunk-level staging.
- Configurable commit conventions, timezone, and intervals.
- Explicit confirmation, saved plans, recovery guidance, and basic Git-state checks.

### Later releases

- Pluggable AI and local-model providers with privacy controls.
- Configurable tests, linting, formatting, and secret-scanning gates.
- Team plan sharing and pull-request summaries.
- Issue tracker, changelog, CI, and pull-request creation integrations.
- Rich terminal interface, editor integration, and analytics for commit-plan quality.

## Working as a two-developer team

We will keep work easy to review and straightforward to merge by using short-lived branches and small, focused commits.

1. Agree on a small issue or task before starting. Assign one owner and record the expected outcome.
2. Each developer creates a branch from the current shared base using a consistent name, such as `feat/…`, `fix/…`, `docs/…`, or `chore/…`.
3. Avoid editing the same files at the same time where practical. If overlap is necessary, agree on file or module ownership first.
4. Use Smart Commit to prepare a commit plan, then review the proposed groups and messages before applying them. Do not rely on automatic grouping without review.
5. Keep commits single-purpose: code with its direct tests is usually one commit; unrelated formatting, documentation, or refactoring gets a separate commit.
6. Push the branch and open a pull request. The other developer reviews the diff, commit messages, and any timestamp changes before merge.
7. Rebase or merge the shared base according to the team convention, resolve conflicts on the feature branch, and run the project checks before requesting final review.
8. Merge only after approval and passing checks. Delete the merged feature branch when it is no longer needed.

### Team agreements

- Treat commit timestamps as intentional metadata. If dates are adjusted, state the reason in the pull request or commit-plan notes.
- Never use changed commit dates to misrepresent authorship, review history, or work performed.
- Do not run `smartcommit apply` on another developer’s uncommitted working tree.
- Prefer one active owner per branch. Pairing is welcome, but coordinate before either person applies the plan or rewrites local commits.
- Keep the default branch protected; all changes reach it through reviewed pull requests.

## Safety principles

- Show a complete dry-run plan before any Git write operation.
- Require explicit confirmation before branch creation, staging, or committing.
- Preserve the working tree if a command fails and explain any recovery steps.
- Make it easy to abort a plan without changing Git state.
- Clearly identify which files and hunks will be included in each commit.
- Validate the repository state, configured rules, and required checks before applying a plan.

## Project status

This repository is currently documenting the intended Smart Commit CLI. The command syntax above is a proposed interface and may evolve as implementation begins.
