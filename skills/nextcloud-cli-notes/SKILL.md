---
name: nextcloud-cli-notes
description: Work with the Nextcloud Notes app through Nextcloud CLI. Use when an agent needs to list notes, create notes, update note content from text or files, delete notes safely, validate Notes app availability, or handle private notes with nxc JSON output and dry-run protection.
---

# Nextcloud CLI notes

Use this skill for `nxc notes ...` operations.

Notes often contain private journal, project, or credential-adjacent material.
Do not dump note contents unless the user explicitly asked for them.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- List without content unless content is needed.
- Prefer `--from-file` for substantial note bodies.
- Use `--dry-run` before create, update, or delete when checking behavior.
- Summarize note ids, titles, categories, and modified times. Avoid raw bodies.

## List notes

```bash
nxc --profile personal notes list --format json
```

If supported by the installed version, exclude bodies for safer summaries:

```bash
nxc --profile personal notes list --exclude-content --format json
```

## Create a note

Preview inline content:

```bash
nxc --profile personal notes create \
  --title "nxc smoke" \
  --content "created by nextcloud-cli dry-run" \
  --category "automation" \
  --dry-run \
  --format json
```

Create from a file:

```bash
nxc --profile personal notes create \
  --title "Project notes" \
  --from-file ./project-notes.md \
  --category "projects" \
  --format json
```

## Update a note

Preview title or content changes:

```bash
nxc --profile personal notes update 123 \
  --title "Updated title" \
  --dry-run \
  --format json
```

Update content from a file:

```bash
nxc --profile personal notes update 123 \
  --from-file ./project-notes.md \
  --format json
```

## Delete a note

Preview first:

```bash
nxc --profile personal notes delete 123 --dry-run --format json
```

Delete only with explicit user intent:

```bash
nxc --profile personal notes delete 123 --yes --format json
```

## Optional app unavailable

If Notes is disabled or missing, report it plainly and continue with other
Nextcloud features. Do not treat optional app absence as setup failure unless
the user specifically needs Notes.

## Smoke workflow

```bash
nxc --profile personal notes list --format json
nxc --profile personal notes create \
  --title "nxc dry-run smoke" \
  --content "dry-run only" \
  --dry-run \
  --format json
```
