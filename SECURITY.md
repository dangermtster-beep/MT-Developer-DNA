# Security Policy

## Supported version

Security fixes are currently provided for the latest `0.8.x` release line.

## Reporting a vulnerability

Do not open a public issue containing exploit details, credentials, tokens, or
private project data. Use GitHub's private vulnerability reporting feature at:

https://github.com/dangermtster-beep/MT-Developer-DNA/security/advisories/new

Include the affected version, operating system, minimal reproduction, impact,
and any suggested remediation. Remove secrets and personal paths from logs.

If private vulnerability reporting is unavailable, open a public issue asking
the maintainers to enable a private contact channel, without vulnerability
details. No response-time SLA is promised for this initial public release.

## Security boundaries

- Deployment is dry-run-first and mutations require explicit approval.
- User-modified provider files are conflicts, not silent overwrite targets.
- Suspected secrets are not copied into memory, reports, or deployment backups.
- Rollback is transaction-scoped and hash-checked.
- MT Developer DNA does not automatically download or install GitHub updates.
- Provider model behavior and third-party CLI security remain outside this repository's trust boundary.

Developed by MT SOFTWARE.
