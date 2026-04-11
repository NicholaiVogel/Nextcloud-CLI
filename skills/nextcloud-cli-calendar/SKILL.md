---
name: nextcloud-cli-calendar
description: Work with Nextcloud Calendar through Nextcloud CLI. Use when an agent needs to list events, create calendar events, delete events safely, validate CalDAV access, or run date-bounded calendar workflows with nxc JSON output and dry-run protection.
---

# Nextcloud CLI calendar

Use this skill for `nxc calendar ...` operations.

Calendar data is private. Prefer narrow date ranges and summarize results.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- Keep ranges small unless the user asks for broad calendar analysis.
- Do not paste full attendee lists, descriptions, or locations unless needed.
- Use `--dry-run` before creating or deleting events.
- Use UTC timestamps or clearly specified local offsets.

## List events

```bash
nxc --profile personal calendar events --range 7d --format json
```

If a specific calendar is needed:

```bash
nxc --profile personal calendar events --calendar personal --range 30d --format json
```

Summarize count, date span, and relevant titles instead of dumping all event
metadata.

## Create an event

Preview first:

```bash
nxc --profile personal calendar create \
  --calendar personal \
  --summary "Project check-in" \
  --starts-at 2026-04-15T16:00:00Z \
  --ends-at 2026-04-15T16:30:00Z \
  --dry-run \
  --format json
```

Create after confirmation:

```bash
nxc --profile personal calendar create \
  --calendar personal \
  --summary "Project check-in" \
  --starts-at 2026-04-15T16:00:00Z \
  --ends-at 2026-04-15T16:30:00Z \
  --format json
```

Optional fields:

```bash
nxc --profile personal calendar create \
  --calendar personal \
  --summary "Project check-in" \
  --starts-at 2026-04-15T16:00:00Z \
  --ends-at 2026-04-15T16:30:00Z \
  --location "Office" \
  --description "Agenda in project notes" \
  --dry-run \
  --format json
```

All-day events:

```bash
nxc --profile personal calendar create \
  --calendar personal \
  --summary "Focus day" \
  --starts-at 2026-04-15 \
  --ends-at 2026-04-16 \
  --all-day \
  --dry-run \
  --format json
```

## Delete an event

List events first to identify the target. Preview deletion if supported by the
current command surface, then delete only with explicit user intent:

```bash
nxc --profile personal calendar delete --calendar personal <event-uid> --dry-run --format json
nxc --profile personal calendar delete --calendar personal <event-uid> --yes --format json
```

If the CLI asks for confirmation or policy changes, stop and ask the user.

## Smoke workflow

```bash
nxc --profile personal calendar events --range 7d --format json
nxc --profile personal calendar create \
  --calendar personal \
  --summary "nxc dry-run smoke" \
  --starts-at 2026-04-15T16:00:00Z \
  --ends-at 2026-04-15T16:15:00Z \
  --dry-run \
  --format json
```

A dry-run create is enough for routine agent validation.
