---
name: nextcloud-cli-files
description: Use Nextcloud CLI to list, search, inspect, create, upload, download, move, rename, copy, and safely delete files through WebDAV. Use when an agent needs file access with nxc, JSON output, path-safe operations, dry-run validation, or read/write file workflows against a configured Nextcloud profile.
---

# Nextcloud CLI files

Use this skill for Nextcloud file operations through `nxc files ...`.

## Core rules

- Prefer `nxc --profile <profile>` for every command.
- Use `--format json` for machine-readable output.
- Treat remote paths as private. Summarize unless the user asks for raw output.
- Use read-only commands before writes.
- Use `--dry-run` before destructive actions.
- Never delete `/` or broad top-level folders.
- Use a dedicated test folder such as `/nextcloud-cli-smoke` for smoke work.

## Read files and folders

List a directory:

```bash
nxc --profile personal files list / --format json
nxc --profile personal files list /Documents --format json
```

Inspect one path:

```bash
nxc --profile personal files stat /Documents/report.pdf --format json
```

Search by name:

```bash
nxc --profile personal files search report \
  --path /Documents \
  --limit 20 \
  --search-mode name \
  --format json
```

Prefer `jq` for local filtering instead of dumping large trees:

```bash
nxc --profile personal files list /Documents --format json | jq '.items[] | {name, path, type, size}'
```

## Visual media search

The files skill also supports reference-image search against an explicit local
media index:

```bash
nxc --profile personal index build --path /Projects --media all --format json
nxc --profile personal files search-image ./reference-frame.png \
  --path /Projects --media all --limit 25 --format json
nxc --profile personal index update --path /Projects --format json
nxc --profile personal index status --format json
```

Indexing is never implicit. Use `index clear --yes` to remove the selected
profile's local index. Image matching uses perceptual fingerprints; video matching
samples frames with `ffmpeg` and refines the strongest candidates around their
coarse timestamps.
## Create folders

Preview or create a folder:

```bash
nxc --profile personal files mkdir /nextcloud-cli-smoke --parents --dry-run --format json
nxc --profile personal files mkdir /nextcloud-cli-smoke --parents --format json
```

Use `--parents` when creating nested folders.

## Upload

Upload to a specific remote path:

```bash
nxc --profile personal files upload ./local.txt /nextcloud-cli-smoke/local.txt --format json
```

Overwrite only when the user asked for it:

```bash
nxc --profile personal files upload ./local.txt /nextcloud-cli-smoke/local.txt --overwrite --format json
```

Set a content type when useful:

```bash
nxc --profile personal files upload ./data.json /nextcloud-cli-smoke/data.json \
  --content-type application/json \
  --format json
```

## Download

Download to an explicit local path:

```bash
nxc --profile personal files download /nextcloud-cli-smoke/local.txt ./local.downloaded.txt --format json
```

Overwrite local files only when the user asked for it:

```bash
nxc --profile personal files download /nextcloud-cli-smoke/local.txt ./local.downloaded.txt --overwrite --format json
```

## Move, rename, and copy

Preview a server-side transfer before executing it:

```bash
nxc --profile personal files move /incoming/PLATES /input/PLATES --dry-run --format json
nxc --profile personal files copy /input/PLATES /archive/PLATES --dry-run --format json
nxc --profile personal files rename /archive/PLATES FINAL --dry-run --format json
```

The commands send WebDAV `MOVE` or `COPY` requests. Directory transfers stay
server-side, and `rename` keeps the destination in the source directory. Existing
destinations are protected by default; pass `--overwrite` only when replacement
is intended:

```bash
nxc --profile personal files move /incoming/report.md /archive/report.md --overwrite --format json
```

Dry-run output includes the resolved absolute source and destination DAV URLs and
does not contact the server.

## Delete safely

Always preview first:

```bash
nxc --profile personal files delete /nextcloud-cli-smoke/local.txt --dry-run --format json
```

Then delete only with explicit user intent:

```bash
nxc --profile personal files delete /nextcloud-cli-smoke/local.txt --yes --format json
```

If the CLI rejects a delete because profile policy denies destructive actions,
show the policy and ask before changing it:

```bash
nxc profiles policy show personal --format json
nxc profiles policy set personal --allow-destructive true
```

## Smoke workflow

```bash
printf 'nextcloud-cli smoke test\n' > /tmp/nxc-smoke.txt
nxc --profile personal files mkdir /nextcloud-cli-smoke --parents --format json
nxc --profile personal files upload /tmp/nxc-smoke.txt /nextcloud-cli-smoke/nxc-smoke.txt --format json
nxc --profile personal files stat /nextcloud-cli-smoke/nxc-smoke.txt --format json
nxc --profile personal files download /nextcloud-cli-smoke/nxc-smoke.txt /tmp/nxc-smoke.downloaded.txt --format json
cmp /tmp/nxc-smoke.txt /tmp/nxc-smoke.downloaded.txt
nxc --profile personal files delete /nextcloud-cli-smoke/nxc-smoke.txt --dry-run --format json
```

Clean up only when permitted:

```bash
nxc --profile personal files delete /nextcloud-cli-smoke/nxc-smoke.txt --yes --format json
```
