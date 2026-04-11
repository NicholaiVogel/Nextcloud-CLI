---
name: nextcloud-cli-contacts
description: Work with Nextcloud Contacts through Nextcloud CLI. Use when an agent needs to search address books, create contacts, delete contacts safely, validate CardDAV access, or run privacy-preserving contact workflows with nxc JSON output and dry-run protection.
---

# Nextcloud CLI contacts

Use this skill for `nxc contacts ...` operations.

Contact data is private. Search narrowly and summarize results.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- Do not dump address books into chat.
- Search by specific name, email, or a harmless smoke query.
- Use `--dry-run` before creating or deleting contacts.
- Do not print phone numbers or email addresses unless the user needs them.

## Search contacts

```bash
nxc --profile personal contacts search Ada --format json
```

Harmless no-match smoke query:

```bash
nxc --profile personal contacts search zzzz-nextcloud-cli-smoke-no-match --format json
```

Summarize matches by count and confidence. Only include raw fields that matter to
the user's task.

## Create a contact

Preview first:

```bash
nxc --profile personal contacts create \
  --addressbook contacts \
  --full-name "Ada Lovelace" \
  --email ada@example.com \
  --dry-run \
  --format json
```

Create after confirmation:

```bash
nxc --profile personal contacts create \
  --addressbook contacts \
  --full-name "Ada Lovelace" \
  --email ada@example.com \
  --phone "+1-555-0100" \
  --organization "Example" \
  --format json
```

## Delete a contact

Search first, identify exactly one target, preview when supported, then delete
only after user confirmation:

```bash
nxc --profile personal contacts delete --addressbook contacts <contact-uid> --dry-run --format json
nxc --profile personal contacts delete --addressbook contacts <contact-uid> --yes --format json
```

If the search returns multiple plausible matches, ask the user to choose.

## Smoke workflow

```bash
nxc --profile personal contacts search zzzz-nextcloud-cli-smoke-no-match --format json
nxc --profile personal contacts create \
  --addressbook contacts \
  --full-name "nxc dry-run smoke" \
  --email nxc-smoke@example.invalid \
  --dry-run \
  --format json
```

A dry-run create plus a no-match search validates CardDAV access without adding
real contact data.
