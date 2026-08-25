# Usage Guide

This guide holds the practical command details for `nxc` and `nextcloud-cli`.
The README gives the product overview. This document is for setup, workflows,
and troubleshooting.

Both command names are supported:

- `nxc`, short form
- `nextcloud-cli`, explicit long form

The examples below use `nxc`. Replace it with `nextcloud-cli` if you prefer the
long command name.

## Quick start

```bash
nxc auth login \
  --server https://cloud.example.com \
  --profile personal

nxc --profile personal server status
nxc --profile personal files list /
```

That gives you a stored app password, a saved profile, and a JSON listing of
your root folder.

## TLS for self-hosted servers

For a server using an internal CA, pass a PEM bundle for the command or set the
environment variable:

```bash
nxc --ca-bundle /path/to/nextcloud-ca.pem --profile personal server status
NEXTCLOUD_CLI_CA_BUNDLE=/path/to/nextcloud-ca.pem nxc --profile personal files list /
```

The custom bundle is added to the system trust store. Hostname verification is
still enforced, so the server URL must match the certificate SAN. Untrusted
issuer, hostname mismatch, and handshake failures return stable TLS error codes
with an actionable hint. Use `--insecure` only as an explicit emergency
override; it disables both checks, prints a warning, is not persisted, and is
rejected for agent-managed profiles.

## Authentication

All authentication flows end with the same result: a Nextcloud app password
stored through the credential backend. Account passwords are not written to
`config.json` or normal command output.

| Situation | Command |
| --- | --- |
| Local desktop with browser access | `auth login` |
| SSH or headless server with account password available outside chat | `auth app-password` |
| CI or existing app password | `auth add` |

### Local desktop

```bash
nxc auth login \
  --server https://cloud.example.com \
  --profile personal
```

`nxc` starts Nextcloud Login Flow v2, opens the approval URL in your browser,
waits for approval, then stores the returned app password.

### Headless or SSH

Use `auth app-password` when there is no browser on the machine running the CLI.
Provide the account password through stdin or an environment variable supplied by
a secret manager. The CLI mints and stores an app password.

```bash
read -rsp "Nextcloud password: " NC_PASSWORD; echo
printf '%s' "$NC_PASSWORD" | nxc auth app-password \
  --server https://cloud.example.com \
  --user you \
  --profile personal \
  --password-stdin
unset NC_PASSWORD
```

If a secret manager injects the account password as an environment variable,
pass the variable name:

```bash
nxc auth app-password \
  --server https://cloud.example.com \
  --user you \
  --profile personal \
  --password-env NC_PASSWORD
```

### Existing app password

```bash
NEXTCLOUD_APP_PASSWORD=... nxc auth add \
  --server https://cloud.example.com \
  --user you \
  --profile personal
```

### Verify authentication

```bash
nxc --profile personal auth status
nxc --profile personal server capabilities --refresh
```

## Implemented commands

Generated command metadata is available from the CLI:

```bash
nxc commands schema --format json
```

Current command groups:

| Area | Commands |
| --- | --- |
| Metadata | `commands schema` |
| Config | `config path`, `config show`, `config doctor` |
| Auth | `auth login`, `auth app-password`, `auth add`, `auth status` |
| Profiles | `profiles list`, `profiles show <name>`, `profiles set-default <name>`, `profiles policy show/set/reset` |
| Server | `server status`, `server capabilities [--refresh]` |
| Files | `files list`, `files search`, `files search-image`, `files stat`, `files mkdir`, `files upload`, `files download`, `files delete` |
| Index | `index status`, `index build`, `index update`, `index clear --yes` |
| Shares | `shares list`, `shares create --public`, `shares delete`, `shares revoke` |
| Calendar | `calendar events`, `calendar create`, `calendar delete` |
| Contacts | `contacts search`, `contacts create`, `contacts delete` |
| Activity | `activity recent` |
| Notes | `notes list`, `notes create`, `notes update`, `notes delete` |
| Deck | `deck boards`, `deck boards create`, `deck stacks create`, `deck cards`, `deck cards create`, `deck cards update`, `deck cards move/archive/delete` |
| Smoke | `smoke run` |
| Updates | `update check` |

See [`COMMANDS.md`](COMMANDS.md) for the generated command list and
[`SPEC.md`](SPEC.md) for the full target surface.

## Visual media search

Visual search is intentionally client-side and explicit. Build a per-profile index
before querying it; normal file listing and name search never crawl or download
media implicitly.

```bash
nxc --profile personal index build \
  --path /Projects \
  --media all \
  --video-sample-rate 1 \
  --format json

nxc --profile personal files search-image ./reference-frame.png \
  --path /Projects \
  --media all \
  --limit 25 \
  --format json

nxc --profile personal index update --path /Projects --format json
nxc --profile personal index status --format json
nxc --profile personal index clear --yes --format json
```

Image files are compared with local perceptual fingerprints. Videos are sampled
with `ffmpeg`, then the best video candidates are refined around their coarse
match timestamps. Video indexing requires `ffmpeg` in `PATH`. The index stores
metadata and fingerprints under the local cache directory; original media is only
held in temporary files while indexing or refining a result.


`config.json` stores profile metadata only. App passwords are stored through the
credential backend.

By default, `nxc` uses `keyring-auto`:

1. Try the operating-system keyring.
2. Fall back to an owner-only local credential file when no usable keyring is
   available.

Force a backend when needed:

```bash
NEXTCLOUD_CLI_KEYRING_BACKEND=keyring nxc auth status
NEXTCLOUD_CLI_KEYRING_BACKEND=file nxc auth status
```

The local file backend writes to:

```text
$XDG_CONFIG_HOME/nextcloud-cli/credentials.json
```

On Unix, that file is created with `0600` permissions.

## Profile selection

Every implemented command resolves its profile in this order:

1. `--profile <name>`
2. `NEXTCLOUD_CLI_PROFILE`
3. stored default profile

Examples:

```bash
nxc --profile personal files list /
NEXTCLOUD_CLI_PROFILE=personal nxc files list /
nxc profiles set-default personal
```

## JSON output

JSON is the default format. Success responses are command-specific. Errors use
a stable envelope:

```json
{
  "error": {
    "code": "confirmation_required",
    "message": "destructive command requires confirmation; pass --yes to continue or --dry-run to preview"
  }
}
```

Write commands include the selected profile and server in their output so
callers can confirm which account was touched.

`--format human` exists, but currently prints the same structured output as
JSON. Richer human formatting is planned.

## Common workflows

### Inspect files

```bash
nxc --profile personal files list /
nxc --profile personal files stat /Documents/report.pdf
nxc --profile personal files search report --path /
```

`files list` returns entries under `entries`:

```bash
nxc --profile personal files list / | jq '.entries[] | {path, name, size}'
```

`files search` returns matches under `files`:

```bash
nxc --profile personal files search report --path / | jq '.files[] | {path, name, size}'
```

### Round-trip a file

```bash
nxc --profile personal files mkdir /nextcloud-cli-smoke --parents
nxc --profile personal files upload ./summary.md /nextcloud-cli-smoke/summary.md
nxc --profile personal files download /nextcloud-cli-smoke/summary.md ./summary.downloaded.md
```

Downloads stream to a temporary file first, verify `Content-Length` when the
server provides it, then rename into place after the write completes.

### Delete safely

```bash
nxc --profile personal files delete /nextcloud-cli-smoke --dry-run
nxc --profile personal files delete /nextcloud-cli-smoke --yes
```

Deletes reject the root path and require either `--dry-run` or `--yes`.

### Generate command metadata

```bash
nxc commands schema --format json | jq '.commands[].name'
```

This is the main integration point for agents that need to discover available
commands without parsing help text.

### List shares

```bash
nxc --profile personal shares list
nxc --profile personal shares list --path /Documents/report.pdf
nxc --profile personal shares list --shared-with-me
```

`shares list` normalizes Nextcloud's OCS response into common share objects and
does not create, modify, or revoke shares.

### Preview public link creation

Public link creation is sensitive. Previewing does not make a network request:

```bash
nxc --profile personal shares create /Documents/report.pdf --public --dry-run
```

Actual public link creation requires both `--yes` and a profile policy that
allows public shares.

### Revoke shares

Share deletion is destructive and sensitive. Previewing does not make a network
request:

```bash
nxc --profile personal shares delete 123 --dry-run
nxc --profile personal shares revoke 123 --dry-run
```

Actual revocation requires `--yes`:

```bash
nxc --profile personal shares revoke 123 --yes
```

## Environment variables

Implemented environment variables:

| Variable | Description |
| --- | --- |
| `NEXTCLOUD_CLI_PROFILE` | Default profile name when `--profile` is not passed. |
| `NEXTCLOUD_CLI_CONFIG_DIR` | Override the config directory. |
| `NEXTCLOUD_CLI_KEYRING_BACKEND` | `keyring`, `file`, or unset for automatic keyring with local-file fallback. |
| `NEXTCLOUD_APP_PASSWORD` | Consumed by `auth add --app-password`. |
| `NEXTCLOUD_CLI_CA_BUNDLE` | PEM CA bundle added to system trust roots for the current invocation. |
| `NEXTCLOUD_CLI_LOG_FILE` | Append secret-redacted JSONL audit events for write commands to this file. |
| `NEXTCLOUD_CLI_AUDIT` | Set to `1` or `true` to write daily JSONL audit logs under the CLI audit directory. |

`auth app-password --password-env <NAME>` can read the account password from
any environment variable name you provide.

## Profile policy

Profiles carry local safety policy. This is separate from Nextcloud server
permissions and exists so scripts and agents can have a smaller blast radius.

```bash
nxc profiles policy show personal
nxc profiles policy set personal --allow-public-shares true
nxc profiles policy reset personal --yes
```

The first implemented policy fields are:

- `agent_mode`
- `default_dry_run`
- `allow_destructive`
- `allow_public_shares`

## Exit codes

Implemented exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | General error. |
| `2` | CLI validation, profile selection, credential selection, path validation, or confirmation error. |
| `3` | HTTP, authentication, or network request failure. |
| `6` | OCS server error envelope. |
| `10` | Login Flow v2 timeout. |

The spec defines a richer exit-code contract for future command families.

## Troubleshooting

### `confirmation_required` on delete

Preview the delete or confirm it explicitly:

```bash
nxc --profile personal files delete /old-stuff --dry-run
nxc --profile personal files delete /old-stuff --yes
```

### Keyring unavailable over SSH or in a container

Force the owner-only file backend:

```bash
NEXTCLOUD_CLI_KEYRING_BACKEND=file nxc auth status
```

The local file backend stores app passwords at:

```text
$XDG_CONFIG_HOME/nextcloud-cli/credentials.json
```

### Login Flow v2 times out

Re-run the login command and approve the browser prompt before the timeout:

```bash
nxc auth login --server https://cloud.example.com --profile personal
```

On a headless machine, use `auth app-password`.

### No profile selected

Pass a profile, set an environment default, or store a default profile:

```bash
nxc profiles list
nxc --profile personal auth status
NEXTCLOUD_CLI_PROFILE=personal nxc auth status
nxc profiles set-default personal
```

### General diagnostics

```bash
nxc config doctor
nxc config show
nxc --profile personal auth status
nxc --profile personal server status
```

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Useful smoke commands, with an authenticated profile:

```bash
nxc --profile personal server capabilities --refresh
nxc --profile personal files mkdir /nextcloud-cli-smoke --parents
nxc --profile personal files upload ./fixture.txt /nextcloud-cli-smoke/fixture.txt
nxc --profile personal files search fixture --path /nextcloud-cli-smoke
nxc --profile personal files download /nextcloud-cli-smoke/fixture.txt ./fixture.downloaded
nxc --profile personal files delete /nextcloud-cli-smoke --yes
```

More detailed smoke instructions live in [`SMOKE.md`](SMOKE.md).
