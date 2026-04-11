---
Repo: github.com/NicholaiVogel/Nextcloud-CLI
Primary language: Rust
Canonical spec: docs/SPEC.md
Canonical agent file: AGENTS.md
Compatibility shim: CLAUDE.md -> AGENTS.md
Last Updated: 2026-04-10
---

This repository builds an unofficial client-side Nextcloud CLI for humans,
shell scripts, and AI agents. The product goal is a safe, JSON-first command
surface over existing Nextcloud WebDAV, OCS, CalDAV, CardDAV, Notes, Deck, and
Activity APIs.

Prefer durable, spec-aligned, maintainable changes over local fixes or
convenience hacks.

## Required workflow

1. Read `docs/SPEC.md` before implementing a new command surface.
2. Keep `README.md`, `docs/COMMANDS.md`, and `docs/SPEC.md` aligned with any
   user-facing behavior change.
3. Add tests for every new command, parser, safety check, and bug fix.
4. Run the full validation suite before committing:

   ```bash
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   git diff --check
   ```

5. When safe and relevant, run a real smoke test against the configured
   Nextcloud profile. Only touch clearly named smoke-test artifacts.

## Safety rules

- Do not print, log, commit, or repeat app passwords, account passwords,
  authorization headers, public share tokens, or private contact/calendar
  details.
- Do not run destructive live-server commands against existing user data.
- Public share creation must stay guarded by `--yes` and profile policy.
- Destructive commands must support `--dry-run` where specified and require
  explicit confirmation for actual execution.
- Audit logs must be JSONL, secret-redacted, and must never corrupt stdout JSON.
- Prefer empty-result live smoke tests for contacts/calendar unless a dedicated
  fixture exists.

## Command naming

- The primary binary is `nextcloud-cli`.
- The short alias is `nxc`.
- Both names should continue to work.

## Commit guidance

- Use conventional commits.
- Reserve `feat:` for user-facing features.
- Use `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, or `perf:` where
  appropriate.
- Keep commits reasonably atomic. Good examples from this repo:
  - `feat: add guarded share deletion`
  - `feat: add profile policy commands`
  - `feat: add write audit events`

## Phase map

- Phase 2: WebDAV file core, implemented.
- Phase 3: sharing and safety policy, implemented for public share
  create/list/delete/revoke and profile policy commands.
- Phase 4: calendar and contacts, implemented for list/search, create, and
  delete. Update commands and deeper compatibility hardening remain planned.
- Phase 5: optional apps, substantially implemented. Activity read, Notes
  list/create/update/delete, Deck boards/cards listing, and Deck board/stack/card
  creation exist. Remaining Deck work is card update/move/archive/delete and
  broader optional-app smoke cleanup.
- Phase 6: distribution and agent experience, partially implemented. README,
  command metadata, CI, `nxc` alias, tagged GitHub release packaging, and
  placeholder `update check` exist. `smoke run` exists for safe redacted
  live-server health reports. npm wrapper, curl installer, real self-update, and
  agent skills remain planned.

When continuing autonomously, complete the next smallest vertical slice,
validate it, smoke it safely, commit it, push it, then update memory.
