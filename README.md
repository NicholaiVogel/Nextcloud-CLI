<h1 align="center">
  <img src="https://nextcloud.com/c/uploads/2025/10/Nextcloud-logo-blue.png?original" alt="Nextcloud" height="72" align="center" />
  &nbsp;nxc
</h1>

<p align="center">
  <strong>An unofficial, client-side Nextcloud CLI for humans, scripts, and AI agents.</strong>
</p>

<p align="center">
  Stable JSON output, secure credential storage, guarded writes, and a practical
  command surface over existing Nextcloud APIs.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/nextcloud-cli"><img alt="npm" src="https://img.shields.io/npm/v/nextcloud-cli?color=0082c9"></a>
  <a href="https://github.com/NicholaiVogel/Nextcloud-CLI/releases"><img alt="GitHub release" src="https://img.shields.io/github/v/release/NicholaiVogel/Nextcloud-CLI?color=0082c9"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

> [!NOTE]
> This is not an officially supported Nextcloud product.

`nxc` talks to existing Nextcloud HTTP APIs from a local machine, SSH session,
CI job, or agent runtime. It authenticates as a normal user and provides a
scriptable handle on files, shares, calendars, contacts, Notes, Deck, Activity,
and raw DAV/OCS commands.

Both command names are supported:

- `nxc`, short form
- `nextcloud-cli`, explicit long form

The examples below use `nxc`. Replace it with `nextcloud-cli` if you prefer the
long command name.

## Contents

- [Status](#status)
- [Install](#install)
- [Quick start](#quick-start)
- [Authentication](#authentication)
- [Implemented commands](#implemented-commands)
- [Credential handling](#credential-handling)
- [Profile selection](#profile-selection)
- [JSON output](#json-output)
- [Common workflows](#common-workflows)
- [Environment variables](#environment-variables)
- [Exit codes](#exit-codes)
- [Troubleshooting](#troubleshooting)
- [Development](#development)
- [Roadmap](#roadmap)
- [Documentation](#documentation)
- [License](#license)

## Status

`nxc` is usable today and published as both GitHub release binaries and an npm
package. It is still pre-1.0, so command details may change as the product
hardens.

| Area | Current support |
| --- | --- |
| Auth | Login Flow v2, headless app-password minting, existing app-password registration |
| Credentials | OS keyring with owner-only file fallback for SSH/headless environments |
| Profiles | create, list, inspect, default selection, local safety policy |
| Server | status and capabilities |
| Files | list, search, stat, mkdir, upload, streaming download, guarded delete |
| Shares | list, public-link dry-run/create, guarded delete/revoke |
| Calendar | event listing, create, guarded delete |
| Contacts | search, create, guarded delete |
| Optional apps | Activity, Notes, and Deck commands |
| Agents | JSON output, command schema, redacted smoke reports |


## Install

### npm

The npm package is the easiest path for most agent and SSH environments. It
downloads the matching GitHub release archive, verifies the SHA-256 checksum,
and exposes both command names.

```bash
npm install -g nextcloud-cli
nxc --help
```

### curl

The curl installer uses the latest GitHub release and installs into
`$HOME/.local/bin` by default.

```bash
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | sh
```

Choose a directory or version when needed:

```bash
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | INSTALL_DIR=/usr/local/bin sh
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | NEXTCLOUD_CLI_INSTALL_VERSION=0.1.0 sh
```

### GitHub Releases

Download a native archive from
[`releases`](https://github.com/NicholaiVogel/Nextcloud-CLI/releases), verify
the matching `.sha256` file, and put both binaries on your `PATH`.

### Source

```bash
git clone https://github.com/NicholaiVogel/Nextcloud-CLI.git
cd Nextcloud-CLI
cargo install --path crates/nextcloud-cli --locked
```

Run from a checkout without installing:

```bash
cargo run -p nextcloud-cli --bin nxc -- <command>
cargo run -p nextcloud-cli -- <command>
```

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

## Authentication

All authentication flows end with the same result: a Nextcloud app password
stored through the credential backend. Account passwords are not written to
`config.json` or normal command output.

| Situation | Command |
| --- | --- |
| Local desktop with browser access | `auth login` |
| SSH or headless server with account password available | `auth app-password` |
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
It reads the account password from stdin, mints an app password through OCS,
then stores only the app password:

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

Current commands:

| Area | Commands |
| --- | --- |
| Metadata | `commands schema` |
| Config | `config path`, `config show`, `config doctor` |
| Auth | `auth login`, `auth app-password`, `auth add`, `auth status` |
| Profiles | `profiles list`, `profiles show <name>`, `profiles set-default <name>`, `profiles policy show/set/reset` |
| Server | `server status`, `server capabilities [--refresh]` |
| Files | `files list`, `files search`, `files stat`, `files mkdir`, `files upload`, `files download`, `files delete` |
| Shares | `shares list`, `shares create --public`, `shares delete`, `shares revoke` |
| Calendar | `calendar events`, `calendar create`, `calendar delete` |
| Contacts | `contacts search`, `contacts create`, `contacts delete` |
| Activity | `activity recent` |
| Notes | `notes list`, `notes create`, `notes update`, `notes delete` |
| Deck | `deck boards`, `deck boards create`, `deck stacks create`, `deck cards`, `deck cards create`, `deck cards update`, `deck cards move/archive/delete` |
| Smoke | `smoke run` |
| Updates | `update check` |

See [`docs/COMMANDS.md`](docs/COMMANDS.md) for the implemented command list and
[`docs/SPEC.md`](docs/SPEC.md) for the full target surface.

## Credential handling

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
allows public shares. New profiles default to `allow_public_shares = false`
until policy commands land.

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

Additional environment variables are specified in
[`docs/SPEC.md`](docs/SPEC.md) and will be documented here as they are
implemented.

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

More detailed smoke instructions live in [`docs/SMOKE.md`](docs/SMOKE.md).

## Roadmap

The current focus is distribution polish and agent experience:

- Homebrew tap
- real `update apply`
- Linux arm64, musl Linux, and Windows arm64 release assets
- setup and workflow skills for coding agents
- broader compatibility hardening across Nextcloud server versions and apps

## Documentation

- [`docs/COMMANDS.md`](docs/COMMANDS.md), implemented command surface
- [`docs/CONFIG.md`](docs/CONFIG.md), configuration and credential behavior
- [`docs/INSTALL.md`](docs/INSTALL.md), installation notes
- [`docs/NETWORK.md`](docs/NETWORK.md), network, proxy, and TLS behavior
- [`docs/SMOKE.md`](docs/SMOKE.md), manual smoke testing
- [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md), real-server compatibility
- [`CONTRIBUTING.md`](CONTRIBUTING.md), contribution guidelines
- [`SECURITY.md`](SECURITY.md), vulnerability reporting
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md), community standards

## License

MIT. See [`LICENSE`](LICENSE).
