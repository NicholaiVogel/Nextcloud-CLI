---
name: nextcloud-cli-activity
description: Read the Nextcloud Activity feed through Nextcloud CLI. Use when an agent needs recent account activity, safe read-only audit context, optional Activity app validation, or summarized activity signals from a configured Nextcloud profile using nxc JSON output.
---

# Nextcloud CLI activity

Use this skill for `nxc activity ...` operations.

Activity is read-only, but it can expose private filenames, collaborators, app
usage, and timing. Summarize unless raw data is required.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- Keep `--limit` small by default.
- Do not paste full activity feeds into chat.
- Treat filenames, share recipients, and timestamps as private metadata.
- If Activity is unavailable, report it as an optional-app absence, not a core
  setup failure.

## Read recent activity

```bash
nxc --profile personal activity recent --limit 20 --format json
```

Use a lower limit for quick health checks:

```bash
nxc --profile personal activity recent --limit 5 --format json
```

## Summarize safely

Good summary shape:

```text
Activity is available. The latest 20 entries include file updates, share changes,
and calendar activity. The most recent entry is from today.
```

Avoid exposing full paths, collaborators, or private content unless the user
asked for the exact feed.

## Smoke workflow

```bash
nxc --profile personal activity recent --limit 5 --format json
```

If the command succeeds, Activity integration is available. If it returns an
optional-app error, continue with other smoke checks.
