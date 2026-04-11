# Security Policy

`nxc` is an unofficial client-side Nextcloud CLI. It stores app passwords,
talks to user-owned Nextcloud servers, and is designed to be usable by scripts
and agents. Security reports are taken seriously.

## Supported versions

The project has not reached v1.0 yet. Security fixes are applied to the main
branch first. Tagged releases will receive explicit support windows once the
release process exists.

## Reporting a vulnerability

Please do not disclose vulnerabilities publicly until they have been reviewed.

Preferred reporting path:

1. Use GitHub's private vulnerability reporting for this repository if it is
   available.
2. If private reporting is unavailable, open a minimal public issue asking for
   a private security contact. Do not include exploit details, credentials,
   server URLs, logs containing secrets, or proof-of-concept payloads in that
   issue.

Useful report details:

- affected commit, tag, or branch
- operating system and install method
- Nextcloud server version if relevant
- clear reproduction steps
- expected impact
- whether credentials, app passwords, files, or server-side state are exposed

## Secret handling expectations

Security reports and examples must not include real credentials. Redact:

- account passwords
- app passwords
- bearer tokens
- cookies
- private server URLs when they identify a private deployment
- file contents unless the content is necessary to demonstrate the issue

## Scope

In scope:

- credential storage or leakage
- command output that exposes secrets
- unsafe handling of local files
- unsafe handling of remote WebDAV or OCS paths
- TLS, proxy, or network behavior that weakens user security
- agent-facing behavior that could cause unintended writes or destructive
  actions

Out of scope:

- vulnerabilities in a user's Nextcloud server or third-party apps
- social engineering
- denial-of-service testing against public or private Nextcloud deployments
  without permission
- reports that require access to someone else's account or infrastructure

## Project posture

This project is under active development. Please expect fixes to land as normal
commits on main until a formal release and advisory process is established.
