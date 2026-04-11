# nextcloud-cli

Unofficial client-side CLI for Nextcloud, built for humans, shell scripts, and AI
agents that need structured access to a user's cloud from outside the server.

`nextcloud-cli` talks to existing Nextcloud HTTP APIs and prints JSON by default.
It is designed for local machines, SSH sessions, CI jobs, and agent runtimes
where predictable output and safe credential handling matter.

Short alias: `nxc`

## Status

This project is in the first implementation pass. The current CLI can:

- authenticate with Nextcloud Login Flow v2
- authenticate headlessly by minting an app password through OCS
- store credentials in the operating-system keyring when available
- fall back to an owner-only local credential file in headless environments
- manage profiles
- inspect server status and capabilities
- list, search, stat, create, upload, download, and delete files through WebDAV
- print machine-readable command metadata for agents

The canonical product contract lives in [`docs/SPEC.md`](docs/SPEC.md).

## Install from source

```bash
git clone https://github.com/NicholaiVogel/Nextcloud-CLI.git
cd Nextcloud-CLI
cargo build --workspace
```

During development, run commands with:

```bash
cargo run -p nextcloud-cli -- <command>
```

The examples below use the installed command name. If you are working from
source, replace `nextcloud-cli` with:

```bash
cargo run -p nextcloud-cli --
```

## Quick start

### 1. Check local configuration

```bash
nextcloud-cli config doctor
nextcloud-cli config path
```

### 2. Authenticate

For a local desktop session, use Login Flow v2:

```bash
nextcloud-cli auth login \
  --server https://cloud.example.com \
  --profile personal
```

For SSH or other headless sessions, read the account password from stdin and
store only the generated app password:

```bash
read -rsp "Nextcloud password: " NC_PASSWORD; echo
printf '%s' "$NC_PASSWORD" | nextcloud-cli auth app-password \
  --server https://cloud.example.com \
  --user you \
  --profile personal \
  --password-stdin
unset NC_PASSWORD
```

If you already have a Nextcloud app password:

```bash
NEXTCLOUD_APP_PASSWORD=... nextcloud-cli auth add \
  --server https://cloud.example.com \
  --user you \
  --profile personal
```

### 3. Verify the profile

```bash
nextcloud-cli --profile personal auth status
nextcloud-cli --profile personal server status
nextcloud-cli --profile personal server capabilities --refresh
```

### 4. Work with files

```bash
nextcloud-cli --profile personal files list /
nextcloud-cli --profile personal files search report --path /
nextcloud-cli --profile personal files stat /Documents/report.pdf
```

Create a folder and round-trip a file through the server:

```bash
nextcloud-cli --profile personal files mkdir /nextcloud-cli-smoke --parents
nextcloud-cli --profile personal files upload ./summary.md /nextcloud-cli-smoke/summary.md
nextcloud-cli --profile personal files download /nextcloud-cli-smoke/summary.md ./summary.downloaded.md
```

Delete requires explicit confirmation:

```bash
nextcloud-cli --profile personal files delete /nextcloud-cli-smoke --dry-run
nextcloud-cli --profile personal files delete /nextcloud-cli-smoke --yes
```

## Implemented command surface

Generated command metadata is available from the CLI:

```bash
nextcloud-cli commands schema --format json
```

Current commands:

| Area | Commands |
| --- | --- |
| Metadata | `commands schema` |
| Config | `config path`, `config show`, `config doctor` |
| Auth | `auth login`, `auth app-password`, `auth add`, `auth status` |
| Profiles | `profiles list`, `profiles show <name>`, `profiles set-default <name>` |
| Server | `server status`, `server capabilities [--refresh]` |
| Files | `files list`, `files search`, `files stat`, `files mkdir`, `files upload`, `files download`, `files delete` |
| Updates | `update check` |

See [`docs/COMMANDS.md`](docs/COMMANDS.md) for the current command list and
[`docs/SPEC.md`](docs/SPEC.md) for the full target surface.

## Credential handling

`config.json` stores profile metadata only. App passwords are stored through the
credential backend.

By default, the CLI uses `keyring-auto`:

1. try the operating-system keyring
2. fall back to an owner-only local credential file when no usable keyring is
   available

Force a backend when needed:

```bash
NEXTCLOUD_CLI_KEYRING_BACKEND=keyring nextcloud-cli auth status
NEXTCLOUD_CLI_KEYRING_BACKEND=file nextcloud-cli auth status
```

The local file backend writes:

```text
$XDG_CONFIG_HOME/nextcloud-cli/credentials.json
```

On Unix, that file is written with `0600` permissions.

## Profile selection

Implemented commands select a profile in this order:

1. `--profile <name>`
2. `NEXTCLOUD_CLI_PROFILE`
3. stored default profile

Examples:

```bash
nextcloud-cli --profile personal files list /
NEXTCLOUD_CLI_PROFILE=personal nextcloud-cli files list /
nextcloud-cli profiles set-default personal
```

## JSON output

JSON is the default output format. Success responses are command-specific.
Errors use a stable envelope:

```json
{
  "error": {
    "code": "confirmation_required",
    "message": "destructive command requires confirmation; pass --yes to continue or --dry-run to preview"
  }
}
```

Write commands include the selected profile and server in their output so agents
can confirm which account they touched.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Useful smoke commands:

```bash
nextcloud-cli --profile personal server capabilities --refresh
nextcloud-cli --profile personal files mkdir /nextcloud-cli-smoke --parents
nextcloud-cli --profile personal files upload ./fixture.txt /nextcloud-cli-smoke/fixture.txt
nextcloud-cli --profile personal files search fixture --path /nextcloud-cli-smoke
nextcloud-cli --profile personal files download /nextcloud-cli-smoke/fixture.txt ./fixture.downloaded
nextcloud-cli --profile personal files delete /nextcloud-cli-smoke --yes
```

More detailed smoke instructions live in [`docs/SMOKE.md`](docs/SMOKE.md).

## Roadmap

The next phase is sharing and safety policy:

- OCS client and envelope parser
- `shares list`
- public link creation with dry-run support
- share revoke/delete with confirmation
- per-profile policy enforcement
- audit event shape for writes

Later phases cover calendar, contacts, Notes, Deck, Activity, packaging, npm
distribution, the installer, and agent skills.

## Documentation

- [`docs/SPEC.md`](docs/SPEC.md), canonical product spec
- [`docs/COMMANDS.md`](docs/COMMANDS.md), implemented command surface
- [`docs/CONFIG.md`](docs/CONFIG.md), configuration and credential behavior
- [`docs/SMOKE.md`](docs/SMOKE.md), manual smoke testing
- [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md), real-server compatibility

## License

MIT
