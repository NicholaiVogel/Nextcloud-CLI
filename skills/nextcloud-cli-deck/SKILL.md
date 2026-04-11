---
name: nextcloud-cli-deck
description: Work with the Nextcloud Deck app through Nextcloud CLI. Use when an agent needs to list boards and cards, create boards or stacks, create or update cards, move cards, archive or delete cards safely, validate Deck availability, or perform dry-run protected project-board workflows with nxc.
---

# Nextcloud CLI Deck

Use this skill for `nxc deck ...` operations.

Deck data is project-management data. It can reveal priorities, client names,
deadlines, and private context. Summarize carefully.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- List boards and stacks before creating or moving cards.
- Use `--dry-run` before write and destructive actions.
- Do not archive or delete cards without explicit user intent.
- If Deck is unavailable, report that the optional app is not enabled.

## List boards and cards

```bash
nxc --profile personal deck boards --format json
nxc --profile personal deck cards --board 1 --format json
```

Use the returned board and stack ids in later commands.

## Create a board or stack

Preview board creation:

```bash
nxc --profile personal deck boards create \
  --title "Automation smoke" \
  --dry-run \
  --format json
```

Preview stack creation:

```bash
nxc --profile personal deck stacks create \
  --board 1 \
  --title "Doing" \
  --dry-run \
  --format json
```

Run without `--dry-run` only when the user asked to create the board or stack.

## Create a card

```bash
nxc --profile personal deck cards create \
  --board 1 \
  --stack 2 \
  --title "Follow up" \
  --description "Check status and report back" \
  --due-at 2026-04-20T18:00:00Z \
  --dry-run \
  --format json
```

After confirmation:

```bash
nxc --profile personal deck cards create \
  --board 1 \
  --stack 2 \
  --title "Follow up" \
  --description "Check status and report back" \
  --format json
```

## Update or move a card

Update title, description, due date, or order:

```bash
nxc --profile personal deck cards update 123 \
  --board 1 \
  --stack 2 \
  --title "Follow up with notes" \
  --dry-run \
  --format json
```

Move between stacks:

```bash
nxc --profile personal deck cards move 123 \
  --board 1 \
  --from-stack 2 \
  --to-stack 3 \
  --dry-run \
  --format json
```

## Archive or delete a card

Archive is usually safer than delete, but still changes the board. Preview first:

```bash
nxc --profile personal deck cards archive 123 \
  --board 1 \
  --stack 2 \
  --dry-run \
  --format json
```

Delete only when specifically requested:

```bash
nxc --profile personal deck cards delete 123 \
  --board 1 \
  --stack 2 \
  --dry-run \
  --format json
```

Execute confirmed actions by removing `--dry-run` only after the user approves.

## Smoke workflow

```bash
nxc --profile personal deck boards --format json
nxc --profile personal deck cards --board 1 --format json
nxc --profile personal deck cards create \
  --board 1 \
  --stack 1 \
  --title "nxc dry-run smoke" \
  --dry-run \
  --format json
```

A dry-run card create is enough for routine Deck validation.
