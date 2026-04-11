---
name: nextcloud-cli-shares
description: Manage Nextcloud shares through Nextcloud CLI. Use when an agent needs to list shares, preview public link creation, create a guarded share, delete or revoke a share, inspect share policy, or handle sensitive share operations safely with nxc.
---

# Nextcloud CLI shares

Use this skill for `nxc shares ...` operations.

Shares are sensitive. Public links can expose private files, and revocation can
remove someone else's access. Move slower than usual here.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for automation.
- List and inspect before mutating.
- Use `--dry-run` before creating or deleting shares.
- Do not create public shares unless the user explicitly asked for a public link.
- Do not print share URLs, tokens, or passwords unless the user needs them.
- Keep profile policy denying public shares unless the user chooses otherwise.

## List shares

```bash
nxc --profile personal shares list --format json
nxc --profile personal shares list --path /Documents/report.pdf --format json
nxc --profile personal shares list --shared-with-me --format json
```

Use `--include-tags` only when the caller needs tag metadata:

```bash
nxc --profile personal shares list --include-tags --format json
```

## Check policy

Public share creation may be blocked by local profile policy:

```bash
nxc profiles policy show personal --format json
```

Only enable public shares with explicit user intent:

```bash
nxc profiles policy set personal --allow-public-shares true
```

## Create a public link

Preview first:

```bash
nxc --profile personal shares create /Documents/report.pdf --public --dry-run --format json
```

Create only after the user confirms the target path and exposure:

```bash
nxc --profile personal shares create /Documents/report.pdf --public --yes --format json
```

Add password and expiry when requested:

```bash
nxc --profile personal shares create /Documents/report.pdf --public \
  --password "$SHARE_PASSWORD" \
  --expire-date 2026-05-01 \
  --yes \
  --format json
```

Unset temporary share secrets:

```bash
unset SHARE_PASSWORD
```

## Delete or revoke shares

Preview revocation:

```bash
nxc --profile personal shares delete 123 --dry-run --format json
nxc --profile personal shares revoke 123 --dry-run --format json
```

Then revoke only with explicit confirmation:

```bash
nxc --profile personal shares delete 123 --yes --format json
nxc --profile personal shares revoke 123 --yes --format json
```

`delete` and `revoke` both remove share access. Use whichever term matches the
user's language.

## Safe response pattern

When reporting results, say what happened without leaking tokens:

- Good: "Created one public share for `/Documents/report.pdf`; it expires on
  2026-05-01."
- Bad: pasting the full public URL into chat by default.

If the user asks for the link, provide it once and remind them it is sensitive.
