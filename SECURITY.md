# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.0   | Yes       |

## Reporting a Vulnerability

If you discover a security vulnerability in SmartCommit, please report it responsibly.

**Do not** open a public issue.

Instead, please send an email to the project maintainers or use the private vulnerability reporting mechanism if one is available.

Please include:
- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact
- Any suggested fixes (if applicable)

We will acknowledge receipt within 48 hours and provide regular updates on our progress.

## Security Considerations

### Design Principles

SmartCommit is designed with security as a primary concern:

1. **No Remote Execution**: SmartCommit does not execute code from remote sources
2. **No Network Access**: The tool operates entirely locally without network calls
3. **Minimal Dependencies**: Dependencies are added only when necessary and are vetted
4. **No AI by Default**: The MVP operates without AI, avoiding LLM-related security concerns
5. **Explicit Confirmation**: All destructive operations require user confirmation (or explicit `--yes` flag)
6. **Safety Checkpoints**: Every operation creates a recovery point before making changes
7. **Secret Protection**: No secrets or API keys are stored in operation records

### Data Privacy

SmartCommit:

- Never sends code or commit data to external services (in the MVP)
- Stores only metadata (commit hashes, timestamps, branch names) in operation records
- Does not store source code or secrets
- Uses temporary files that are cleaned up after operations
- Respects `.gitignore` and does not access untracked files unless explicitly requested

### Git Safety

SmartCommit's Git operations are designed to be safe:

- Uses temporary index files to preserve the real working tree
- Never uses destructive commands like `git reset --hard` without explicit user intent
- Validates repository state before operations (detects conflicts, merge/rebase in progress)
- Creates checkpoint refs before making changes
- Refuses to proceed if HEAD has moved since checkpoint
- Hooks are respected and their failures are treated as execution failures

### Future AI Integration

When AI features are implemented (future releases), additional security considerations will apply:

- API keys will be stored securely (environment variables or encrypted config)
- Code sent to AI providers will be clearly disclosed
- Local LLM option will be available for sensitive codebases
- AI will be optional and fallback to deterministic matching will always be available

### Dependency Security

SmartCommit uses a minimal set of Rust dependencies:

- `clap` - CLI argument parsing
- `thiserror` - Error handling
- `chrono` - Date/time handling
- `serde` - Serialization
- `toml` - Configuration parsing

Dependencies are audited before inclusion and updated regularly for security patches.

### File System Access

SmartCommit:

- Operates only within the specified repository
- Requires explicit `--repo` flag for non-current repositories
- Does not access files outside the repository tree
- Respects file system permissions
- Uses temporary directories under `.git/smartcommit/tmp/` that are cleaned up

### Attack Surface

The current MVP has a minimal attack surface:

- No network access
- No remote code execution
- No plugin system
- No user-uploaded content processing
- No privilege escalation (runs with user's permissions)

### Best Practices for Users

To use SmartCommit securely:

1. **Review the Plan**: Always review `scm plan` output before running `scm commit`
2. **Use Dry Run**: Test with `--dry-run` first if unsure
3. **Keep Backups**: While SmartCommit has restore capability, regular backups are still recommended
4. **Check Hooks**: Review your Git hooks to understand what they do
5. **Verify Commits**: After running SmartCommit, verify the resulting commits with `git log`
6. **Use Version Control**: Keep your repository in good standing before using SmartCommit

### Supply Chain Security

SmartCommit is designed to be built from source:

- Source code is available for review
- Build process uses standard Cargo tooling
- No pre-compiled binaries are distributed from untrusted sources
- Users can verify builds by compiling from source

### Vulnerability Response Process

When a vulnerability is reported:

1. Acknowledge receipt within 48 hours
2. Validate the vulnerability
3. Develop a fix
4. Test the fix thoroughly
5. Release a security update
6. Publish security advisory
7. Update documentation as needed

### Security Audits

SmartCommit has not yet undergone a formal security audit.

We welcome security reviews from the community. If you're interested in conducting a security audit, please contact us first to coordinate.

### License and Liability

SmartCommit is provided as-is without warranty. See LICENSE for details.

The maintainers are not responsible for any data loss or damage resulting from the use of this software. Users should always:

- Keep backups of important repositories
- Test on non-critical repositories first
- Review commit plans before execution
- Use `--dry-run` to preview changes

## Security Questions

If you have security questions or concerns that don't involve a vulnerability report, please open an issue with the `security` label.
