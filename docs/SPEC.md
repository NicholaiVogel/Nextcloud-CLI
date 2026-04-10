# Nextcloud CLI Product Spec

Status: Draft product specification  
Primary binary: `nextcloud-cli`  
Short alias: `nxc`  
Target milestone: Issue-complete MVP  
Reference issue: <https://github.com/nextcloud/server/issues/59417>

## How to Use This Spec

This file is the canonical product contract for the first implementation pass. It
is intentionally more specific than a normal planning document because the work is
likely to be delegated across agents and sessions. When implementation reveals a
Nextcloud API detail that differs from this document, update this spec in the same
change that updates the code.

The spec should be read in three layers:

1. Product contract: sections 1 through 8 define what the CLI is, who it serves,
   and what every command must guarantee.
2. Feature contracts: sections 9 through 27 define the public command surface,
   output shapes, safety rules, and completion gates.
3. Delivery contract: sections 28 through 42 define fixtures, tests,
   architecture, compatibility, distribution, documentation, and release
   readiness.

The first implementation should move in thin vertical slices. Auth, profiles,
config, HTTP client, WebDAV file listing, JSON output, and error envelopes come
before optional app depth. That gives the project a working spine before the
ribs go on. No need to hang curtains before the house has studs.


## Implementation Status Snapshot

Last updated: 2026-04-10 after the Phase 1 continuation work.

The repository now has a working Rust workspace and an initial executable CLI.
The current implementation covers the repository spine, Login Flow v2, manual
app-password auth, profile/config plumbing, server status/capability calls with a
per-profile cache, and the first WebDAV file commands. The GitHub repository exists at
<https://github.com/NicholaiVogel/Nextcloud-CLI> and `main` tracks
`origin/main`.

Completed so far:

- Rust workspace with `nextcloud` library crate and `nextcloud-cli` binary crate.
- `nextcloud-cli` binary plus `nxc` development alias.
- JSON output by default for implemented commands.
- Stable JSON error envelope for CLI/runtime errors.
- Config directory resolution with `NEXTCLOUD_CLI_CONFIG_DIR` and `--config-dir`.
- Profile config schema with schema version, default profile, server, username,
  credential reference, and safe default policy fields.
- Credential backend abstraction with an initial local file backend storing app
  passwords outside `config.json` with owner-only file permissions on Unix.
- Login Flow v2 setup through `auth login`, including `--no-open` for headless
  use.
- Manual app-password setup through `auth add`.
- `auth status`, `profiles list`, `profiles show`, and `profiles set-default`.
- `server status` through `status.php`.
- `server capabilities` through OCS cloud capabilities, with `--refresh` and
  per-profile cache support.
- Initial WebDAV client with PROPFIND and MKCOL support.
- Multistatus XML parser with unit coverage.
- `files list`, `files stat`, and `files mkdir --dry-run`.
- Command metadata through `commands schema`.
- README plus `docs/COMMANDS.md`, `docs/CONFIG.md`, `docs/INSTALL.md`,
  `docs/NETWORK.md`, `docs/SMOKE.md`, and `docs/COMPATIBILITY.md`.
- GitHub Actions CI for fmt, clippy, and tests.

Validation that passed after the current implementation:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -q -p nextcloud-cli -- --version
cargo run -q -p nextcloud-cli --bin nxc -- --version
```

Real-server smoke started against `https://nextcloud.biohazardvfx.com`:

- `status.php` succeeded and reported Nextcloud `29.0.1`.
- Login Flow v2 start succeeded and produced an approval URL.
- Authenticated smoke is still pending browser approval.

Important pending items before MVP:

- Complete real-server Login Flow v2 approval smoke.
- OS keyring backend validation on target desktop/server platforms.
- Broader real-server smoke validation across auth, capabilities, and files.
- Mock HTTP tests for WebDAV and OCS commands.
- `files upload`, `files download`, and file search.
- Shares, calendar, contacts, notes, Deck, activity, raw DAV/OCS commands.
- Full profile policy commands and enforcement.
- Distribution work: release artifacts, npm wrapper, curl installer, and real
  self-update.

## 1. Product Summary

`nextcloud-cli` is a local command line client for Nextcloud, built for humans,
shell scripts, and AI agents that need structured access to a user's personal
cloud from outside the Nextcloud server.

Nextcloud already has `occ`, but `occ` is a server-side administration tool. This
product is a client-side CLI. It authenticates as a user, talks to existing
Nextcloud HTTP APIs, and returns stable JSON that agents can consume without
custom integrations.

The MVP implements the full surface requested in Nextcloud server issue #59417:

- files: list, search, download, upload
- shares: list, create public share link
- calendar: events for today or a date range
- contacts: search
- notes: list
- deck: boards and cards
- activity: recent activity feed

The product also includes safe write operations for the personal productivity
surfaces agents are likely to manage on a user's behalf: calendar events,
contacts, notes, and Deck boards/stacks/cards. These write operations are gated
by dry-run behavior, profile policy, structured output, and cleanup tests.

The implementation should follow the spirit of `references/cli`, the Google
Workspace CLI reference in this repository:

- Rust workspace with a reusable library crate and a thin CLI crate
- structured JSON output by default
- useful human output formats where safe
- explicit auth commands
- secure credential storage
- dry-run support
- stable errors
- testable command contracts
- agent-friendly examples and skills

Nextcloud does not have a single Google Discovery-style API catalog for all of
these surfaces. This product is therefore hybrid: curated first-class commands
for the requested workflows, plus raw `dav` and `ocs` escape hatches for long-tail
automation.

## 2. Goals

1. Satisfy the exact user-facing command needs in issue #59417.
2. Provide a client-side CLI that works from a laptop, server, CI job, or agent
   runtime.
3. Make every normal command return JSON by default.
4. Store user credentials securely using the system keyring where available.
5. Use existing Nextcloud APIs rather than requiring a new server app.
6. Give AI agents safe, predictable primitives for interacting with personal
   cloud data.
7. Keep command behavior deterministic enough for tests, scripts, and model
   tool use.
8. Support safe write operations for calendars, contacts, notes, and Deck without
   making destructive changes easy by accident.
9. Provide hard completion gates so implementation can be delegated without
   product ambiguity.

## 3. Non-goals

The MVP does not include:

- a file sync engine
- a replacement for server-side `occ`
- a patch to `nextcloud/server`
- server administration commands
- group, user, app, quota, or provisioning administration
- OIDC bearer auth
- background daemon behavior
- filesystem watching
- conflict resolution for sync
- web UI components

These may be future work, but they are not required for the issue-complete MVP.

## 4. Source References

Local references:

- `references/cli`: Google Workspace CLI style and architecture reference.
- `references/server`: local Nextcloud server source checkout.
- `references/server/core/openapi.json`: Login Flow v2 OpenAPI definitions.
- `references/server/core/Controller/ClientFlowLoginV2Controller.php`: Login Flow
  v2 controller behavior.
- `references/server/apps/dav/openapi.json`: DAV OCS endpoints, including
  upcoming events.
- `references/server/apps/dav/lib`: WebDAV, CalDAV, CardDAV, and search source.
- `references/server/apps/files_sharing/openapi.json`: OCS Share API OpenAPI.
- `references/server/apps/files_sharing/appinfo/routes.php`: share route source.

External references:

- Nextcloud Login Flow v2:
  <https://docs.nextcloud.com/server/latest/developer_manual/client_apis/LoginFlow/index.html>
- Nextcloud WebDAV:
  <https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html>
- Nextcloud OCS overview:
  <https://docs.nextcloud.com/server/stable/developer_manual/client_apis/OCS/ocs-api-overview.html>
- Nextcloud OCS Share API:
  <https://docs.nextcloud.com/server/stable/developer_manual/client_apis/OCS/ocs-share-api.html>
- Nextcloud Notes API v1:
  <https://raw.githubusercontent.com/nextcloud/notes/master/docs/api/v1.md>
- Nextcloud Deck API:
  <https://github.com/nextcloud/deck/blob/master/docs/API.md>
- Nextcloud Activity API v2:
  <https://github.com/nextcloud/activity/blob/master/docs/endpoint-v2.md>
- Nextcloud Search developer documentation:
  <https://docs.nextcloud.com/server/latest/developer_manual/digging_deeper/search.html>
- Nextcloud reference providers and unified search OCS API notes:
  <https://docs.nextcloud.com/server/latest/developer_manual/digging_deeper/reference.html>
- Nextcloud FullTextSearch OCS collection API:
  <https://docs.nextcloud.com/server/stable/developer_manual/client_apis/OCS/ocs-fulltextsearch-collections-api.html>

Implementation should prefer local server references for built-in server APIs.
For optional apps that are not bundled in `references/server`, use the app's own
GitHub documentation or source as the primary reference.

### 4.1 Google Workspace CLI lessons to preserve

When product or implementation details are uncertain, inspect `references/cli`
before inventing a new pattern. The reference CLI is a working implementation
with several decisions this project should deliberately keep unless Nextcloud
forces a different shape:

- a Rust workspace with a reusable library crate and a thin binary crate
- a small npm wrapper that downloads GitHub Release binaries instead of shipping
  compiled binaries inside the npm package
- platform detection in `npm/platform.js`, including Linux musl detection
- checksum verification in the npm installer before extraction
- `run.js` fallback behavior that reinstalls if the binary is missing
- tag-driven GitHub Release publication with one archive and one checksum per
  supported target
- release notes that include direct installation commands
- Cargo publishing after native binaries are available
- agent skill documentation that treats installation as part of the product
- JSON output and stable command contracts as the agent-facing surface

If this spec diverges from `references/cli`, the divergence should be intentional
and documented in the implementation PR. The useful question is: what did that
project already learn the hard way?

## 5. User Stories and Use Cases

### 5.1 AI agent file lookup

As an AI agent, I can search a user's Nextcloud files before answering a
question, then download relevant files when needed.

Example:

```bash
nextcloud-cli files search "tax return" \
  | jq '.files[] | {path, name, mime_type, modified_at}'
```

Required outcome:

- The agent receives JSON.
- The JSON includes enough metadata to decide whether to download a file.
- The command does not expose credentials in stdout, stderr, logs, or dry-runs.

### 5.2 AI agent file transfer

As an AI agent, I can upload generated artifacts and download existing user files.

Examples:

```bash
nextcloud-cli files download /Documents/brief.md ./brief.md
nextcloud-cli files upload ./summary.md /Documents/summary.md
```

Required outcome:

- Upload and download paths are explicit.
- Binary transfers do not corrupt data.
- Metadata is returned as JSON after successful transfer.

### 5.3 Public share creation

As an AI agent, I can create a public share link for a user-approved file or
folder.

Example:

```bash
nextcloud-cli shares create /Documents/report.pdf --public
```

Required outcome:

- The response includes the public URL.
- The response includes share id, token, permissions, expiration state, and
  password state.
- Default permissions are safe and documented.

### 5.4 Calendar awareness

As an AI agent, I can check today's calendar events or events over the next seven
days.

Examples:

```bash
nextcloud-cli calendar events --date today
nextcloud-cli calendar events --range 7d
```

Required outcome:

- The response includes start and end timestamps.
- All-day events are clearly marked.
- Recurring events are expanded enough for the requested window.

### 5.5 Contact lookup

As an AI agent, I can search contacts by name, email, or organization.

Example:

```bash
nextcloud-cli contacts search "Amari"
```

Required outcome:

- The response includes useful contact fields without exposing more than needed.
- Multiple address books can be searched.
- Empty results are valid JSON, not an error.

### 5.6 Notes lookup

As an AI agent, I can list Nextcloud Notes for reference or summarization.

Example:

```bash
nextcloud-cli notes list --exclude-content
```

Required outcome:

- The command works when the Notes app is installed.
- Missing Notes app produces a structured unavailable-app error.
- Large note collections can be chunked or limited.

### 5.7 Deck task lookup

As an AI agent, I can inspect Deck boards and cards.

Examples:

```bash
nextcloud-cli deck boards
nextcloud-cli deck cards --board 10
```

Required outcome:

- Boards are returned as JSON.
- Cards can be flattened from stacks for easier agent consumption.
- Missing Deck app produces a structured unavailable-app error.

### 5.8 Activity feed lookup

As an AI agent, I can inspect recent activity.

Example:

```bash
nextcloud-cli activity recent --limit 20
```

Required outcome:

- Activity items are returned in a normalized JSON shape.
- The command supports a limit.
- Missing Activity app produces a structured unavailable-app error.

### 5.9 Human scripting

As a human user, I can use the CLI in shell scripts without special parsing.

Example:

```bash
nextcloud-cli files list /Invoices --format json \
  | jq -r '.entries[] | select(.mime_type == "application/pdf") | .path'
```

Required outcome:

- JSON is stable.
- Errors are parseable.
- Exit codes distinguish validation, auth, network, unavailable app, and server
  failures.

### 5.10 Multi-organization operation

As an agent or power user, I can manage more than one Nextcloud account without
confusing personal, client, and organization data.

Examples:

```bash
nextcloud-cli profiles list
nextcloud-cli files list /Documents --profile personal
nextcloud-cli files list /Shared --profile client-a
```

Required outcome:

- Profile selection is explicit and testable.
- Write and sensitive command outputs include the selected profile and server.
- Ambiguous profile selection fails before any network request is sent.
- Per-profile policy can restrict paths, commands, transfer sizes, public shares,
  and agent behavior.

## 6. Product Principles

### 6.1 JSON-first

Every normal command prints JSON to stdout by default. Human-readable formats are
allowed, but JSON is the contract.

### 6.2 Agent-safe

The CLI is a tool an agent can call repeatedly. It should avoid interactive
surprises after authentication is configured. It should not emit progress bars,
color codes, or prose on stdout unless explicitly requested.

### 6.3 Credentials stay out of sight

Secrets must never appear in:

- stdout
- stderr
- logs
- dry-run output
- panic messages
- serialized errors
- test snapshots

### 6.4 Curated commands first

The requested workflows deserve polished commands. Raw DAV and OCS commands are
escape hatches, not the primary UX.

### 6.5 Existing APIs only

The MVP must work against existing Nextcloud servers and apps. It must not
require a custom server plugin.

### 6.6 Hard gates

A feature is not complete because the happy path works once. It is complete when
its command contract, test coverage, validation, examples, and failure modes are
all done.

### 6.7 Reference before inventing

When unsure about install flow, release packaging, command ergonomics, agent skill
shape, or JSON contracts, check `references/cli` first. This prevents novelty for
novelty's sake and keeps the Nextcloud CLI aligned with a working CLI that was
already designed for humans and agents.

### 6.8 Terminal identity and ASCII art

The CLI should make heavy use of terminal-native ASCII art and text composition
in human-facing surfaces. This is part of the product identity, not decoration.
Nextcloud is personal infrastructure, so the CLI should feel like a mix of
personal cloud utility and old-internet terminal tool: crafted, local, slightly
weird in the right way, and not SaaS-polished.

Use ASCII art in:

- README hero and install section
- release notes
- top-level `nextcloud-cli --help` when stdout is a TTY
- successful first-run setup
- setup agent skill documentation
- demos, VHS recordings, and docs examples

Do not use ASCII art in subcommand help by default. Subcommand help should be
fast, plain, and easy to scan.

Do not emit ASCII art in:

- JSON mode
- NDJSON stream mode
- command stdout intended for parsing
- non-interactive agent mode
- redirected stdout
- error JSON

Rules:

- ASCII art must never corrupt machine-readable output.
- `--no-color` disables color but not necessarily art.
- `--format json` and `--agent` disable art.
- `NO_COLOR=1` disables color.
- `NEXTCLOUD_CLI_NO_ART=1` disables art globally.
- Use plain ASCII by default for maximum terminal compatibility.
- Box-drawing Unicode may be used in docs or TTY-only output if a fallback exists.
- Do not use emoji in product art or CLI output.
- Keep art narrow enough for normal terminals, target 80 columns or less.
- Use `nxc` as the main block-text mark.
- Use `nextcloud-cli` as the subtitle or explanatory name.

Recommended asset layout:

```text
art/
  intro.txt
  outro.txt
  install.txt
  auth-success.txt
  first-run.txt
  features.txt
  doctor.txt
```

The Google Workspace CLI's `references/cli/art` directory is the style reference
for treating terminal art as part of release and demo material. This project
should adapt the idea without copying the exact visual language. `nxc` should be
the big mark, `nextcloud-cli` should be the subtitle, and the visual language
should mix cloud/file motifs with old-internet terminal craft.

## 7. CLI Surface

### 7.1 Global syntax

```bash
nextcloud-cli <command> [subcommand] [args] [flags]
nxc <command> [subcommand] [args] [flags]
```

The `nxc` alias must behave identically to `nextcloud-cli`.

### 7.2 Global flags

| Flag | Description |
| --- | --- |
| `--profile <name>` | Use a named credential profile. Defaults to `default`. |
| `--format <format>` | Output format. Supported: `json`, `table`, `yaml`, `csv`. Default: `json`. |
| `--dry-run` | Validate and print the request that would be sent without sending it. |
| `--no-color` | Disable color in human output and diagnostics. |
| `--no-art` | Disable ASCII art in human-facing output. |
| `--quiet` | Suppress nonessential stderr diagnostics. |
| `--verbose` | Enable verbose diagnostics on stderr. Secrets still redacted. |
| `--config-dir <path>` | Override config directory. |
| `--ca-bundle <path>` | Use an additional or replacement CA bundle for TLS verification. |
| `--insecure` | Disable TLS verification for this command. Must be explicit and noisy. |
| `--agent` | Use the selected profile's agent policy defaults for this command. |
| `--help` | Show help. |
| `--version` | Show version. |

### 7.3 Environment variables

| Variable | Description |
| --- | --- |
| `NEXTCLOUD_CLI_PROFILE` | Default profile name. |
| `NEXTCLOUD_CLI_CONFIG_DIR` | Override config directory. |
| `NEXTCLOUD_CLI_CREDENTIALS_FILE` | Path to exported credentials JSON. |
| `NEXTCLOUD_URL` | Server URL for env-based auth. |
| `NEXTCLOUD_USER` | Username or login name for env-based auth. |
| `NEXTCLOUD_APP_PASSWORD` | App password for env-based auth. |
| `NEXTCLOUD_CLI_KEYRING_BACKEND` | `keyring` or `file`. Defaults to `keyring`. |
| `NEXTCLOUD_CLI_LOG` | Log level for diagnostics. |
| `NEXTCLOUD_CLI_LOG_FILE` | Optional directory for JSON logs. |
| `NEXTCLOUD_CLI_AGENT_MODE` | When `1` or `true`, apply agent-mode defaults for the selected profile. |
| `NEXTCLOUD_CLI_NO_ART` | When `1` or `true`, disable ASCII art globally. |
| `NEXTCLOUD_CLI_CA_BUNDLE` | Additional or replacement CA bundle path for TLS verification. |
| `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` | Standard proxy configuration honored by HTTP clients. |
| `NEXTCLOUD_CLI_CONCURRENCY` | Optional default concurrency cap for bulk operations. |

### 7.4 Credential precedence

1. `NEXTCLOUD_CLI_CREDENTIALS_FILE`
2. `NEXTCLOUD_URL`, `NEXTCLOUD_USER`, and `NEXTCLOUD_APP_PASSWORD`
3. encrypted local profile selected by `--profile` or `NEXTCLOUD_CLI_PROFILE`

If a higher-precedence source exists but is invalid, the command must fail with a
clear auth error. It must not silently fall through to a lower-precedence source.
Silent fallback makes debugging auth failures ugly as homemade sin.

### 7.5 Profile and account commands

The CLI must treat multi-account usage as a first-class product requirement. A
single user or agent may manage multiple Nextcloud servers across different
organizations, clients, or personal accounts.

Profile commands:

```bash
nextcloud-cli profiles list
nextcloud-cli profiles current
nextcloud-cli profiles use <profile>
nextcloud-cli profiles show <profile>
nextcloud-cli profiles rename <old> <new>
nextcloud-cli profiles remove <profile>
nextcloud-cli profiles policy show <profile>
nextcloud-cli profiles policy set <profile> --agent-mode safe
nextcloud-cli profiles policy reset <profile> --yes
```

Auth commands must create or update profiles:

```bash
nextcloud-cli auth login --server https://personal.example.com --profile personal
nextcloud-cli auth login --server https://cloud.client-a.org --profile client-a
nextcloud-cli auth add --server https://cloud.client-b.org --user agent --profile client-b
```

Profile metadata must include:

```json
{
  "profile": "client-a",
  "server": "https://cloud.client-a.org",
  "login_name": "nicholai",
  "user_id": "nicholai",
  "display_name": "Nicholai",
  "organization": "Client A",
  "auth_type": "app_password",
  "created_at": "2026-04-10T15:04:05Z",
  "last_validated_at": "2026-04-10T15:04:05Z"
}
```

The `organization` field is user-provided metadata. The CLI must not infer an
organization name from the domain unless it is only shown as a suggestion during
interactive setup.

Profile selection rules:

1. `--profile <name>` wins for a single command.
2. `NEXTCLOUD_CLI_PROFILE` wins when no flag is provided.
3. The stored current profile is used when no flag or environment variable is
   provided.
4. If no current profile exists and exactly one profile exists, use it.
5. If multiple profiles exist and no profile is selected, fail with
   `profile_required` and list available profile names.

Commands that perform writes must include the selected profile and server in
their JSON output. This gives agents a cheap confirmation that they touched the
intended Nextcloud instance.

Update commands are part of the public CLI surface from the first release:

```bash
nextcloud-cli update check
nextcloud-cli update apply [--yes]
```

They are specified in the distribution section, but they must appear in command
metadata, help output, tests, and release gates like every other public command.

### 7.6 Implemented command surface

Implemented as of commit `8d3139e`:

```bash
nextcloud-cli commands schema
nextcloud-cli config path
nextcloud-cli config show
nextcloud-cli config doctor
nextcloud-cli auth login --server <url> --profile <name> --no-open
nextcloud-cli auth add --server <url> --user <user> --profile <name> --app-password <password>
nextcloud-cli auth status
nextcloud-cli profiles list
nextcloud-cli profiles show <name>
nextcloud-cli profiles set-default <name>
nextcloud-cli server status
nextcloud-cli server capabilities [--refresh]
nextcloud-cli files list [path]
nextcloud-cli files stat <path>
nextcloud-cli files mkdir <path> --dry-run
nextcloud-cli update check
```

Implementation notes:

- `--format json`, `--profile`, `--config-dir`, `--no-art`, `--help`, and
  `--version` are currently wired.
- `--format human` currently prints the same structured JSON as `json`; richer
  human formatting remains pending.
- Global `--dry-run`, `--no-color`, `--quiet`, `--verbose`, `--ca-bundle`,
  `--insecure`, and `--agent` remain pending. `files mkdir` has command-local
  `--dry-run`.
- `auth login` and `auth add` are implemented. `auth login` has `--no-open` for
  headless use.
- Credential storage uses an abstraction with `keyring-auto`, an OS keyring backend,
  and a local owner-only file fallback for headless environments.
- `server capabilities` uses a per-profile cache. `--refresh` bypasses and
  rewrites the cache.
- `update check` currently reports development placeholder metadata. Release
  discovery and `update apply` remain pending.

### 7.7 Exit codes

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | General error. |
| `2` | CLI validation error. |
| `3` | Authentication error. |
| `4` | Authorization or permission error. |
| `5` | Network or TLS error. |
| `6` | Server returned an error. |
| `7` | Requested optional app is unavailable. |
| `8` | Parse error from malformed XML, JSON, iCalendar, or vCard. |
| `9` | Local file I/O error. |
| `10` | Timeout. |

## 8. Output Contract

### 8.1 Success envelope

Each command may return a command-specific object, but the top-level shape must be
stable per command. Do not wrap every success in a generic envelope unless the
command needs metadata.

List commands should generally return:

```json
{
  "items": [],
  "count": 0
}
```

Commands with domain-specific names may use a clearer collection key:

```json
{
  "files": [],
  "count": 0
}
```

The exact shape for each public command is defined in the feature specs below.

### 8.2 Error envelope

Errors must be printed to stdout as JSON when `--format json` is active or when
stdout is expected to be machine-readable. Human diagnostics may be printed to
stderr only when they do not contain secrets.

```json
{
  "error": {
    "code": "auth_failed",
    "message": "Authentication failed for profile 'default'.",
    "hint": "Run `nextcloud-cli auth login --server https://cloud.example.com`.",
    "status": 401,
    "source": "ocs"
  }
}
```

Required error fields:

- `code`: stable snake_case error code
- `message`: concise human-readable message

Optional error fields:

- `hint`: remediation step
- `status`: HTTP status or OCS status code
- `source`: `cli`, `auth`, `dav`, `ocs`, `caldav`, `carddav`, `notes`, `deck`,
  `activity`, or `filesystem`
- `details`: structured extra context without secrets

### 8.3 Date and time format

All timestamps in CLI-produced JSON must be RFC 3339 strings unless the upstream
API only provides Unix timestamps and the raw value is intentionally preserved.
When preserving raw values, include a normalized field too.

Example:

```json
{
  "modified_at": "2026-04-10T15:04:05Z",
  "modified_unix": 1775833445
}
```

### 8.4 Null and empty values

- Empty collections must be `[]`.
- Unknown optional fields may be `null`.
- Missing optional fields from upstream APIs should be normalized to `null` where
  the field is part of the command contract.

### 8.5 Stdout and stderr

- JSON results go to stdout.
- Progress and diagnostics go to stderr.
- Downloads write file bytes to the requested local path.
- If a future command supports writing binary to stdout, JSON metadata must not be
  mixed into stdout.

## 9. Auth Feature Spec

### 9.1 `auth login`

Syntax:

```bash
nextcloud-cli auth login --server <url> [--profile <name>] [--timeout <duration>] [--no-open]
```

Purpose:

Start Nextcloud Login Flow v2, prompt the user to approve access in a browser,
poll for credentials, then store the returned app password securely.

Backing API:

- `POST /index.php/login/v2`
- `POST /index.php/login/v2/poll`

Behavior:

1. Normalize and validate the server URL.
2. Send `POST /index.php/login/v2`.
3. Read `poll.token`, `poll.endpoint`, and `login` from the response.
4. Open the `login` URL unless `--no-open` is set.
5. Print the login URL to stderr for manual use.
6. Poll `poll.endpoint` with the token until approved, expired, or timed out.
7. Store returned credentials in the selected profile.
8. Validate credentials with `auth status` behavior.
9. Print stored profile metadata as JSON.

Expected output:

```json
{
  "profile": "default",
  "server": "https://cloud.example.com",
  "login_name": "nicholai",
  "user_id": "nicholai",
  "auth_type": "app_password",
  "stored": true
}
```

Errors:

- invalid server URL
- login flow unsupported
- polling timeout
- login flow expired
- user denied access
- credentials could not be stored
- credentials stored but validation failed

Completion gate:

- Unit tests cover init response parsing, poll response parsing, timeout, 404
  pending behavior, and secret redaction.
- Mock integration test covers full login flow.
- Real-server smoke test confirms login works.
- Docs include browser and `--no-open` examples.

### 9.2 `auth add`

Syntax:

```bash
nextcloud-cli auth add --server <url> --user <user> --app-password <password> [--profile <name>]
```

Purpose:

Configure credentials manually for headless or scripted environments.

Behavior:

1. Validate server URL.
2. Validate required user and app-password fields are non-empty.
3. Store credentials securely.
4. Validate credentials.
5. Print profile metadata as JSON.

Expected output:

```json
{
  "profile": "default",
  "server": "https://cloud.example.com",
  "login_name": "nicholai",
  "user_id": "nicholai",
  "auth_type": "app_password",
  "stored": true
}
```

Completion gate:

- Mock integration test validates credentials against capabilities or user status.
- Secret does not appear in process logs, errors, or output.
- Real-server smoke test confirms manual app password works.

### 9.3 `auth status`

Syntax:

```bash
nextcloud-cli auth status [--profile <name>]
```

Purpose:

Show current auth state and verify credentials.

Expected output:

```json
{
  "profile": "default",
  "authenticated": true,
  "server": "https://cloud.example.com",
  "login_name": "nicholai",
  "user_id": "nicholai",
  "capabilities_checked": true
}
```

Completion gate:

- Test valid, missing, invalid, and expired/revoked credentials.

### 9.4 `auth export`

Syntax:

```bash
nextcloud-cli auth export [--profile <name>] [--unmasked]
```

Purpose:

Export credentials for CI or another machine.

Behavior:

- Without `--unmasked`, secrets are masked.
- With `--unmasked`, app password is included.
- Output is JSON.

Completion gate:

- Tests verify masked and unmasked behavior.
- Unmasked export requires explicit flag.

### 9.5 `auth logout`

Syntax:

```bash
nextcloud-cli auth logout [--profile <name>] [--revoke]
```

Purpose:

Delete local credentials and optionally revoke the app password.

Behavior:

- Remove local profile credentials.
- If `--revoke` is set, attempt app-password deletion endpoint where supported.
- If remote revoke fails after local deletion, return warning metadata.

Completion gate:

- Local deletion test.
- Remote revoke success and failure tests.

## 10. Files Feature Spec

### 10.1 Common file object

Normalized file entries should use this shape:

```json
{
  "name": "report.pdf",
  "path": "/Documents/report.pdf",
  "type": "file",
  "mime_type": "application/pdf",
  "size": 123456,
  "etag": "abc123",
  "file_id": 42,
  "permissions": "RDNVW",
  "favorite": false,
  "has_preview": true,
  "owner_id": "nicholai",
  "owner_display_name": "Nicholai",
  "created_at": null,
  "modified_at": "2026-04-10T15:04:05Z"
}
```

`type` must be `file`, `folder`, or `unknown`.

### 10.2 `files list`

Syntax:

```bash
nextcloud-cli files list <path> [--depth 1|infinity]
```

Backing API:

- `PROPFIND /remote.php/dav/files/{username}/{path}`

Behavior:

- Default depth is `1`.
- Root path may be `/`.
- Return entries excluding the collection itself unless `--include-self` is added
  in the future.

Expected output:

```json
{
  "path": "/Documents",
  "entries": [
    {
      "name": "report.pdf",
      "path": "/Documents/report.pdf",
      "type": "file",
      "mime_type": "application/pdf",
      "size": 123456,
      "etag": "abc123",
      "file_id": 42,
      "permissions": "RDNVW",
      "favorite": false,
      "has_preview": true,
      "owner_id": "nicholai",
      "owner_display_name": "Nicholai",
      "created_at": null,
      "modified_at": "2026-04-10T15:04:05Z"
    }
  ],
  "count": 1
}
```

Completion gate:

- Tests cover root listing, nested listing, empty folder, missing folder, malformed
  multistatus XML, and path encoding.
- Real-server smoke test lists a known test folder.

### 10.3 `files search`

Syntax:

```bash
nextcloud-cli files search <query> [--path <scope>] [--limit <n>]
```

Backing API:

- DAV `SEARCH` against `/remote.php/dav` using Nextcloud's file search backend.
- Source reference: `references/server/apps/dav/lib/Files/FileSearchBackend.php`.

Behavior:

- Default scope is `/`.
- Default limit is `25`.
- Search should match display names first.
- Results use the common file object shape.

Expected output:

```json
{
  "query": "report",
  "scope": "/",
  "files": [],
  "count": 0
}
```

Completion gate:

- Tests cover XML request generation, normal results, empty results, invalid scope,
  and unsupported server behavior.
- Real-server smoke test finds an uploaded fixture file.

### 10.4 `files stat`

Syntax:

```bash
nextcloud-cli files stat <path>
```

Backing API:

- `PROPFIND /remote.php/dav/files/{username}/{path}` with `Depth: 0`

Behavior:

- Inspect exactly one file or folder.
- Return the common file object shape.
- Do not download file contents.
- Treat missing paths and permission failures as structured errors.

Expected output:

```json
{
  "file": {
    "name": "report.pdf",
    "path": "/Documents/report.pdf",
    "type": "file",
    "mime_type": "application/pdf",
    "size": 123456,
    "etag": "abc123",
    "file_id": 42,
    "permissions": "RDNVW",
    "modified_at": "2026-04-10T15:04:05Z"
  }
}
```

Completion gate:

- Tests cover file stat, folder stat, missing path, permission denied, malformed
  multistatus XML, and path encoding.
- Real-server smoke test stats an uploaded fixture file.

### 10.5 `files mkdir`

Syntax:

```bash
nextcloud-cli files mkdir <path> [--parents]
```

Backing API:

- `MKCOL /remote.php/dav/files/{username}/{path}`

Behavior:

- Create a remote folder.
- Without `--parents`, fail if the parent folder is missing.
- With `--parents`, create missing parent folders in order.
- Return created path metadata where available.
- Support `--dry-run`.

Expected output:

```json
{
  "path": "/Documents/Generated",
  "created": true,
  "parents_created": []
}
```

Completion gate:

- Tests cover create success, already exists, missing parent, `--parents`, invalid
  path, permission denied, dry-run, and path encoding.
- Real-server smoke test creates the test folder before upload when needed.

### 10.6 `files download`

Syntax:

```bash
nextcloud-cli files download <remote> <local> [--overwrite]
```

Backing API:

- `GET /remote.php/dav/files/{username}/{remote}`

Behavior:

- Refuse to overwrite existing local file unless `--overwrite` is set.
- Create parent directories only if `--parents` is added in the future.
- Write metadata JSON to stdout after successful download.

Expected output:

```json
{
  "remote": "/Documents/report.pdf",
  "local": "./report.pdf",
  "bytes_written": 123456,
  "etag": "abc123"
}
```

Completion gate:

- Tests cover binary integrity, existing local file refusal, 404 remote, permission
  denied, and local I/O failure.
- Real-server smoke test downloads an uploaded fixture file and verifies bytes.

### 10.7 `files upload`

Syntax:

```bash
nextcloud-cli files upload <local> <remote> [--overwrite] [--content-type <mime>]
```

Backing API:

- `PUT /remote.php/dav/files/{username}/{remote}`

Behavior:

- Refuse to overwrite unless `--overwrite` is set or server behavior guarantees
  safe replacement semantics requested by the user.
- Infer content type when possible.
- Return uploaded metadata if available via follow-up `PROPFIND`.

Expected output:

```json
{
  "remote": "/Documents/report.pdf",
  "local": "./report.pdf",
  "bytes_uploaded": 123456,
  "etag": "abc123"
}
```

Completion gate:

- Tests cover upload success, missing local file, local directory input, remote
  conflict, content type, and path encoding.
- Real-server smoke test uploads and verifies via `files list`.

## 11. Shares Feature Spec

### 11.1 Common share object

```json
{
  "id": "123",
  "path": "/Documents/report.pdf",
  "share_type": "public_link",
  "share_with": null,
  "url": "https://cloud.example.com/s/abc123",
  "token": "abc123",
  "permissions": 1,
  "password_protected": false,
  "expiration": null,
  "created_at": "2026-04-10T15:04:05Z"
}
```

### 11.2 `shares list`

Syntax:

```bash
nextcloud-cli shares list [--path <path>] [--shared-with-me] [--include-tags]
```

Backing API:

- `GET /ocs/v2.php/apps/files_sharing/api/v1/shares`

Behavior:

- Always send `OCS-APIRequest: true`.
- Request JSON where supported.
- Normalize OCS response into common share objects.

Expected output:

```json
{
  "shares": [],
  "count": 0
}
```

Completion gate:

- Tests cover no shares, link shares, user shares, path filter, OCS error envelope,
  and permission errors.
- Real-server smoke test lists shares after creating a fixture share.

### 11.3 `shares create --public`

Syntax:

```bash
nextcloud-cli shares create <path> --public [--password <password>] [--expire-date <yyyy-mm-dd>]
```

Backing API:

- `POST /ocs/v2.php/apps/files_sharing/api/v1/shares`

Behavior:

- Public link share maps to Nextcloud share type `3`.
- Default permissions should be read-only unless explicitly expanded later.
- If password is provided, never echo it.
- Return normalized share object.

Expected output:

```json
{
  "share": {
    "id": "123",
    "path": "/Documents/report.pdf",
    "share_type": "public_link",
    "url": "https://cloud.example.com/s/abc123",
    "token": "abc123",
    "permissions": 1,
    "password_protected": false,
    "expiration": null
  }
}
```

Completion gate:

- Tests cover public share creation, password redaction, expiration, missing path,
  forbidden path, OCS validation errors, and malformed OCS response.
- Real-server smoke test creates and deletes or records cleanup of a fixture share.

### 11.4 `shares delete` / `shares revoke`

Syntax:

```bash
nextcloud-cli shares delete <share-id> [--yes]
nextcloud-cli shares revoke <share-id> [--yes]
```

Backing API:

- `DELETE /ocs/v2.php/apps/files_sharing/api/v1/shares/{share-id}`

Behavior:

- `shares revoke` is an alias for `shares delete` if both are implemented.
- Safety class is both `destructive` and `sensitive`.
- Non-interactive contexts require `--yes`.
- Support `--dry-run`.
- Return the deleted share id and selected profile/server.
- If the share id is missing or already gone, return a structured OCS error.

Expected output:

```json
{
  "deleted": true,
  "share_id": "123",
  "profile": "personal",
  "server": "https://cloud.example.com"
}
```

Completion gate:

- Tests cover delete success, revoke alias, missing share, permission denied,
  non-interactive confirmation, dry-run, and OCS error mapping.
- Smoke tests clean up any share created during the run.

## 12. Calendar Feature Spec

### 12.1 Common event object

```json
{
  "uid": "event-uid",
  "calendar": "personal",
  "summary": "Meeting",
  "description": null,
  "location": "Office",
  "starts_at": "2026-04-10T16:00:00Z",
  "ends_at": "2026-04-10T17:00:00Z",
  "all_day": false,
  "status": "CONFIRMED",
  "href": "/remote.php/dav/calendars/nicholai/personal/event-uid.ics",
  "etag": "abc123",
  "organizer": null,
  "attendees": []
}
```

### 12.2 `calendar events`

Syntax:

```bash
nextcloud-cli calendar events --date today
nextcloud-cli calendar events --range 7d
nextcloud-cli calendar events --from <date> --to <date>
```

Backing API:

- CalDAV under `/remote.php/dav/calendars/{username}/`.
- Calendar discovery via `PROPFIND`.
- Event range query via CalDAV `calendar-query` REPORT.

Implementation note:

The local server checkout also exposes
`GET /ocs/v2.php/apps/dav/api/v1/events/upcoming`, but source inspection shows
that endpoint is designed for a limited upcoming-events summary and currently
limits results internally. The CLI should use CalDAV for the issue-requested date
and range behavior.

Behavior:

- `--date today` uses the user's local timezone by default.
- `--range 7d` means from now through seven days from now.
- `--from` and `--to` are explicit date/time bounds.
- Recurring events should be expanded for the requested window where CalDAV
  response provides expansions.

Expected output:

```json
{
  "range": {
    "from": "2026-04-10T00:00:00-06:00",
    "to": "2026-04-11T00:00:00-06:00"
  },
  "events": [],
  "count": 0
}
```

Completion gate:

- Tests cover date parsing, range parsing, timezone handling, empty calendars,
  single event, all-day event, recurring event fixture, cancelled event filtering,
  and malformed iCalendar.
- Real-server smoke test reads events from a test calendar or records an empty
  calendar as valid if discovery succeeds.

### 12.3 `calendar create`

Syntax:

```bash
nextcloud-cli calendar create \
  --calendar <name> \
  --summary <text> \
  --starts-at <datetime> \
  --ends-at <datetime> \
  [--timezone <tz>] \
  [--location <text>] \
  [--description <text>] \
  [--all-day]
```

Backing API:

- CalDAV `PUT` of an iCalendar object under
  `/remote.php/dav/calendars/{username}/{calendar}/`.

Behavior:

- Generate a stable event UID when not provided.
- Serialize a valid `VCALENDAR` with one `VEVENT`.
- Use explicit timezone handling.
- Support `--dry-run` by printing the target href and redacted iCalendar preview.
- Return normalized event metadata including href and ETag when available.

Expected output:

```json
{
  "event": {
    "uid": "event-uid",
    "calendar": "personal",
    "summary": "Meeting",
    "starts_at": "2026-04-10T16:00:00Z",
    "ends_at": "2026-04-10T17:00:00Z",
    "href": "/remote.php/dav/calendars/nicholai/personal/event-uid.ics",
    "etag": "abc123"
  },
  "created": true,
  "profile": "personal",
  "server": "https://cloud.example.com"
}
```

Completion gate:

- Tests cover timed event, all-day event, timezone, missing required fields,
  iCalendar serialization, dry-run, conflict, and permission denied.
- Smoke tests may create a test event only when write smoke tests are enabled.

### 12.4 `calendar update`

Syntax:

```bash
nextcloud-cli calendar update <event-ref> [flags] [--if-match <etag>]
```

`<event-ref>` may be an event UID or href returned by `calendar events` or
`calendar create`.

Behavior:

- Fetch the existing event when needed.
- Apply only provided field changes.
- Preserve fields the CLI does not understand when possible.
- Use `If-Match` when an ETag is available or explicitly provided.
- Return `conflict` if the server rejects the update due to stale ETag.
- Support `--dry-run`.

Completion gate:

- Tests cover summary update, time update, location update, stale ETag conflict,
  unknown event, serialization preservation, and dry-run.

### 12.5 `calendar delete`

Syntax:

```bash
nextcloud-cli calendar delete <event-ref> [--yes] [--if-match <etag>]
```

Behavior:

- Delete the CalDAV event resource.
- Safety class is `destructive`.
- Non-interactive contexts require `--yes`.
- Use `If-Match` when available.
- Support `--dry-run`.

Completion gate:

- Tests cover delete success, unknown event, stale ETag, confirmation requirement,
  permission denied, and dry-run.
- Smoke tests delete any event created during write smoke runs.

## 13. Contacts Feature Spec

### 13.1 Common contact object

```json
{
  "uid": "contact-uid",
  "addressbook": "contacts",
  "full_name": "Ada Lovelace",
  "emails": ["ada@example.com"],
  "phones": [],
  "organization": null,
  "title": null,
  "href": "/remote.php/dav/addressbooks/users/nicholai/contacts/contact-uid.vcf",
  "etag": "abc123"
}
```

### 13.2 `contacts search`

Syntax:

```bash
nextcloud-cli contacts search <name> [--limit <n>]
```

Backing API:

- CardDAV under `/remote.php/dav/addressbooks/users/{username}/`.
- Address book discovery via `PROPFIND`.
- Search via CardDAV `addressbook-query` REPORT.

Behavior:

- Search across all discovered user address books by default.
- Default limit is `25`.
- Empty results are success.

Expected output:

```json
{
  "query": "Ada",
  "contacts": [],
  "count": 0
}
```

Completion gate:

- Tests cover address book discovery, name match, email match, empty results,
  malformed vCard, and permission errors.
- Real-server smoke test searches a known fixture contact or records empty results
  after successful address book discovery.

### 13.3 `contacts create`

Syntax:

```bash
nextcloud-cli contacts create \
  --addressbook <name> \
  --full-name <name> \
  [--email <email>] \
  [--phone <phone>] \
  [--organization <text>]
```

Backing API:

- CardDAV `PUT` of a vCard under
  `/remote.php/dav/addressbooks/users/{username}/{addressbook}/`.

Behavior:

- Generate a contact UID when not provided.
- Serialize a valid vCard.
- Support multiple `--email` and `--phone` flags.
- Support `--dry-run`.
- Return normalized contact metadata including href and ETag when available.

Completion gate:

- Tests cover minimal contact, email/phone arrays, organization, vCard escaping,
  invalid email warning behavior, conflict, permission denied, and dry-run.

### 13.4 `contacts update`

Syntax:

```bash
nextcloud-cli contacts update <contact-ref> [flags] [--if-match <etag>]
```

`<contact-ref>` may be a contact UID or href returned by `contacts search` or
`contacts create`.

Behavior:

- Fetch the existing vCard when needed.
- Apply provided field changes.
- Preserve unsupported vCard properties when possible.
- Use `If-Match` when an ETag is available or explicitly provided.
- Support `--dry-run`.

Completion gate:

- Tests cover full-name update, email replacement, adding phone, stale ETag,
  unknown contact, preservation of unsupported vCard fields, and dry-run.

### 13.5 `contacts delete`

Syntax:

```bash
nextcloud-cli contacts delete <contact-ref> [--yes] [--if-match <etag>]
```

Behavior:

- Delete the CardDAV contact resource.
- Safety class is `destructive`.
- Non-interactive contexts require `--yes`.
- Use `If-Match` when available.
- Support `--dry-run`.

Completion gate:

- Tests cover delete success, unknown contact, stale ETag, confirmation
  requirement, permission denied, and dry-run.

## 14. Notes Feature Spec

### 14.1 Common note object

```json
{
  "id": 76,
  "etag": "be284e00488c61c101ee28309d235e0b",
  "readonly": false,
  "modified_at": "2013-08-17T15:31:04Z",
  "modified_unix": 1376753464,
  "title": "New note",
  "category": "sub-directory",
  "favorite": false,
  "content": "New note\n and something more"
}
```

### 14.2 `notes list`

Syntax:

```bash
nextcloud-cli notes list [--category <name>] [--exclude-content] [--limit <n>]
```

Backing API:

- `GET /index.php/apps/notes/api/v1/notes`

Behavior:

- Use `exclude=content` when `--exclude-content` is set.
- Use Notes chunking parameters where needed for large result sets.
- Normalize Unix `modified` into both `modified_unix` and `modified_at`.
- If the Notes app is unavailable, return `app_unavailable` with source `notes`.

Expected output:

```json
{
  "notes": [],
  "count": 0
}
```

Completion gate:

- Tests cover normal list, category filter, exclude content, chunk cursor, 401,
  404 app unavailable, and malformed JSON.
- Real-server smoke test lists notes when app is enabled or records unavailable
  app as an expected optional-app result.

### 14.3 `notes create`

Syntax:

```bash
nextcloud-cli notes create \
  --title <title> \
  [--content <markdown>] \
  [--from-file <path>] \
  [--category <name>]
```

Backing API:

- Notes app API v1.

Behavior:

- Accept content from `--content`, `--from-file`, or stdin when explicitly
  requested in a future flag.
- Refuse both `--content` and `--from-file` together.
- Do not print full note content in agent-safe mode unless requested.
- Support `--dry-run`.
- If Notes is unavailable, return `app_unavailable` with source `notes`.

Completion gate:

- Tests cover create with content, create from file, category, empty content,
  app unavailable, malformed response, and dry-run.

### 14.4 `notes update`

Syntax:

```bash
nextcloud-cli notes update <note-id> [--title <title>] [--content <markdown>] [--from-file <path>]
```

Behavior:

- Update only provided fields.
- Preserve category and favorite state unless explicitly changed by future flags.
- Support optimistic conflict detection when the API exposes ETag or modified
  metadata.
- Support `--dry-run`.

Completion gate:

- Tests cover title update, content update, from-file update, missing note,
  app unavailable, conflict metadata where supported, and dry-run.

### 14.5 `notes delete`

Syntax:

```bash
nextcloud-cli notes delete <note-id> [--yes]
```

Behavior:

- Delete the note through the Notes API.
- Safety class is `destructive`.
- Non-interactive contexts require `--yes`.
- Support `--dry-run`.

Completion gate:

- Tests cover delete success, missing note, app unavailable, confirmation
  requirement, permission denied, and dry-run.

## 15. Deck Feature Spec

### 15.1 Common board object

```json
{
  "id": 10,
  "title": "Board title",
  "owner_uid": "admin",
  "owner_display_name": "Administrator",
  "color": "ff0000",
  "archived": false,
  "deleted_at": null,
  "permissions": {
    "read": true,
    "edit": true,
    "manage": true,
    "share": true
  }
}
```

### 15.2 Common card object

```json
{
  "id": 81,
  "board_id": 10,
  "stack_id": 5,
  "stack_title": "Doing",
  "title": "Test card",
  "description": null,
  "labels": [],
  "assignees": [],
  "due_at": null,
  "archived": false,
  "deleted_at": null,
  "etag": "bdb10fa2d2aeda092a2b6b469454dc90"
}
```

### 15.3 `deck boards`

Syntax:

```bash
nextcloud-cli deck boards [--details]
```

Backing API:

- `GET /index.php/apps/deck/api/v1.0/boards`

Behavior:

- Send `OCS-APIRequest: true`.
- Send `Content-Type: application/json`.
- If `--details` is set, request detailed board information where supported.
- If Deck is unavailable, return `app_unavailable` with source `deck`.

Expected output:

```json
{
  "boards": [],
  "count": 0
}
```

Completion gate:

- Tests cover board list, empty list, details, permission error, app unavailable,
  and malformed JSON.
- Real-server smoke test lists boards when Deck is enabled or records unavailable
  app as an expected optional-app result.

### 15.4 `deck cards`

Syntax:

```bash
nextcloud-cli deck cards --board <id> [--include-archived]
```

Backing API:

- `GET /index.php/apps/deck/api/v1.0/boards/{boardId}/stacks`

Behavior:

- Fetch stacks for the board.
- Flatten stack card arrays into one `cards` list.
- Preserve stack id and stack title on each card.
- Exclude archived/deleted cards unless `--include-archived` is set.

Expected output:

```json
{
  "board_id": 10,
  "cards": [],
  "count": 0
}
```

Completion gate:

- Tests cover stacked cards, empty stacks, archived filtering, board not found,
  permission error, app unavailable, and malformed JSON.
- Real-server smoke test fetches cards for a known board when Deck is enabled.

### 15.5 Deck write commands

Deck write support should begin with the workflow agents actually need: create a
board if needed, create a stack, create/update/move/archive/delete cards, and
clean up test artifacts. More advanced board sharing and label management can be
added later.

Public commands:

```bash
nextcloud-cli deck boards create --title <title> [--color <hex>]
nextcloud-cli deck stacks create --board <id> --title <title>
nextcloud-cli deck cards create --board <id> --stack <id> --title <title> [--description <markdown>] [--due-at <datetime>]
nextcloud-cli deck cards update <card-id> [flags] [--if-match <etag>]
nextcloud-cli deck cards move <card-id> --board <id> --stack <id> [--order <n>]
nextcloud-cli deck cards archive <card-id> [--yes]
nextcloud-cli deck cards delete <card-id> [--yes]
```

Backing API:

- Deck app API v1.0.

Behavior:

- If Deck is unavailable, return `app_unavailable` with source `deck`.
- Write outputs include selected profile and server.
- Destructive commands require confirmation in non-interactive contexts.
- Use ETag or revision fields for conflict detection where the Deck API exposes
  them.
- Support `--dry-run` for every write command.
- Prefer archive over hard delete where the server exposes both semantics.

Completion gate:

- Tests cover board create, stack create, card create, card update, card move,
  archive/delete confirmation, app unavailable, board not found, stack not found,
  stale ETag where supported, malformed JSON, and dry-run.
- Smoke tests may create and clean up a test board/card only when Deck is enabled
  and write smoke tests are enabled.

## 16. Activity Feature Spec

### 16.1 Common activity object

```json
{
  "id": 12345,
  "app": "files",
  "type": "file_changed",
  "subject": "You changed report.pdf",
  "subject_rich": null,
  "message": null,
  "object_type": "files",
  "object_id": 42,
  "link": "https://cloud.example.com/f/42",
  "icon": null,
  "datetime": "2026-04-10T15:04:05Z",
  "actor": {
    "type": "users",
    "id": "nicholai",
    "name": "Nicholai"
  }
}
```

### 16.2 `activity recent`

Syntax:

```bash
nextcloud-cli activity recent [--limit <n>] [--since <id>] [--filter <filter>]
```

Backing API:

- `GET /ocs/v2.php/apps/activity/api/v2/activity`
- Optional filtered endpoint: `/ocs/v2.php/apps/activity/api/v2/activity/{filter}`

Behavior:

- Default limit is `20`.
- Send `OCS-APIRequest: true`.
- Request JSON format.
- Normalize the Activity OCS response into common activity objects.
- If Activity is unavailable, return `app_unavailable` with source `activity`.

Expected output:

```json
{
  "activities": [],
  "count": 0,
  "next_since": null
}
```

Completion gate:

- Tests cover normal activity list, limit, since, filter, app unavailable, OCS
  error envelope, and malformed JSON.
- Real-server smoke test lists recent activity when Activity is enabled or records
  unavailable app as an expected optional-app result.

## 17. Raw Request Feature Spec

### 17.1 `dav request`

Syntax:

```bash
nextcloud-cli dav request <method> <path> [--body <json-or-xml>] [--header <key:value>]
```

Purpose:

Provide an authenticated DAV escape hatch for advanced users and debugging.

Behavior:

- Resolve relative DAV path against `/remote.php/dav`.
- Preserve custom headers except forbidden auth or host headers.
- Redact auth in dry-run.
- Output raw response body if JSON/XML parsing is not requested.

Completion gate:

- Tests cover method validation, path validation, header redaction, dry-run, and a
  mocked DAV request.

### 17.2 `ocs request`

Syntax:

```bash
nextcloud-cli ocs request <method> <path> [--params <json>] [--json <json>]
```

Purpose:

Provide an authenticated OCS escape hatch.

Behavior:

- Resolve relative path against `/ocs/v2.php` unless an absolute OCS path is
  provided.
- Always send `OCS-APIRequest: true`.
- Request JSON format where supported.
- Parse OCS envelopes when possible.

Completion gate:

- Tests cover GET, POST, OCS envelope parsing, OCS error parsing, dry-run, and
  malformed response fallback.

## 18. Optional App Handling

Notes, Deck, and Activity are optional app surfaces. Calendar and Contacts may be
backed by DAV services even when their web UI apps are disabled.

When an optional app is unavailable, return:

```json
{
  "error": {
    "code": "app_unavailable",
    "message": "The Deck app is not available on this Nextcloud server.",
    "hint": "Enable the Deck app or skip `nextcloud-cli deck` commands for this profile.",
    "status": 404,
    "source": "deck"
  }
}
```

Unavailable optional app behavior is a valid smoke-test result only when the smoke
record explicitly marks that app unavailable. Unit and mock integration tests must
still cover successful behavior.

## 19. Multi-account and Multi-organization Spec

Multi-account support is part of the MVP, not a later convenience. Agents and
power users commonly work across personal clouds, client clouds, and organization
clouds. The CLI must make the active account explicit and hard to confuse.

### 19.1 Account model

A profile represents one login on one Nextcloud server.

Profile identity key:

```text
profile name -> server URL + login name + stored credential
```

The same server may have multiple profiles for different users. The same user may
have profiles on multiple servers. Profile names are local aliases and may be
renamed without changing credentials.

### 19.2 Profile naming

Profile names must:

- be unique
- be case-sensitive or case-insensitive consistently across platforms
- avoid path separators
- avoid control characters
- be safe as config map keys

Recommended names:

```text
personal
client-a
client-b-admin
work
family
```

The CLI should reject ambiguous names like empty strings, `.` and `..`.

### 19.3 Current profile

The CLI may store a current profile for convenience, but it must not hide
ambiguity.

Behavior:

- `profiles use <profile>` sets the current profile.
- `profiles current` prints the current profile metadata.
- `profiles list` marks the current profile.
- If multiple profiles exist and no profile is selected, commands fail unless a
  current profile has been explicitly set.

Expected `profiles list` output:

```json
{
  "current": "personal",
  "profiles": [
    {
      "profile": "personal",
      "server": "https://cloud.example.com",
      "login_name": "nicholai",
      "organization": "Personal",
      "current": true,
      "last_validated_at": "2026-04-10T15:04:05Z"
    }
  ],
  "count": 1
}
```

### 19.4 Agent safety requirements

Agents must be able to confirm which account they are using before taking action.

Requirements:

- Every write command output includes `profile` and `server`.
- `--dry-run` includes `profile` and `server`.
- `auth status` includes `profile`, `server`, and `login_name`.
- `profiles list` never exposes secrets.
- `profiles show` never exposes secrets unless a future explicit export command
  is used.
- If a command fails because multiple profiles are available, the error includes
  available profile names and a hint to pass `--profile`.

Example ambiguity error:

```json
{
  "error": {
    "code": "profile_required",
    "message": "Multiple Nextcloud profiles are configured and no profile was selected.",
    "hint": "Pass `--profile personal` or run `nextcloud-cli profiles use personal`.",
    "source": "cli",
    "details": {
      "profiles": ["personal", "client-a", "client-b"]
    }
  }
}
```

### 19.5 Organization metadata

A profile may include optional organization metadata:

- `organization`
- `purpose`
- `notes`
- `tags`

This metadata is local only. It is useful for agents deciding which account to
use, but it must not be sent to the Nextcloud server.

Example:

```bash
nextcloud-cli profiles show client-a
```

```json
{
  "profile": "client-a",
  "server": "https://cloud.client-a.org",
  "login_name": "agent-service",
  "organization": "Client A",
  "purpose": "client document access",
  "tags": ["client", "read-mostly"]
}
```

### 19.6 Environment-variable profiles

Environment variables may define credentials without storing a profile. When they
are used, the effective profile name should be `env` unless
`NEXTCLOUD_CLI_PROFILE` is set.

If environment credentials are active, `profiles list` should not pretend they are
stored credentials. It may include an `effective` block:

```json
{
  "current": null,
  "profiles": [],
  "effective": {
    "profile": "env",
    "server": "https://cloud.example.com",
    "login_name": "agent"
  },
  "count": 0
}
```

### 19.7 Multi-account completion gate

Multi-account support is complete only when:

- `profiles list`, `profiles current`, `profiles use`, `profiles show`,
  `profiles rename`, and `profiles remove` are implemented.
- Multiple profiles can target the same server with different users.
- Multiple profiles can target different servers.
- Commands fail clearly when profile selection is ambiguous.
- `--profile` overrides current profile.
- `NEXTCLOUD_CLI_PROFILE` overrides current profile.
- Environment credentials work without being stored.
- Write-command outputs include selected `profile` and `server`.
- Tests cover profile selection precedence, ambiguity, rename, removal, and
  secret redaction.



## 20. Per-profile Policy and Agent Mode Spec

Per-profile policy is part of the MVP. Nextcloud app passwords can be broad, and
agents are very good at doing exactly what they were asked to do against the
wrong scope if the tool lets them. Local policy gives each profile a safety
boundary independent of server-side permissions.

### 20.1 Policy commands

Public commands:

```bash
nextcloud-cli profiles policy show <profile>
nextcloud-cli profiles policy set <profile> [flags]
nextcloud-cli profiles policy reset <profile> [--yes]
```

Minimum `policy set` flags:

```bash
--agent-mode off|safe|strict
--read-only true|false
--allow-command <family>
--deny-command <family>
--allow-path <remote-prefix>
--deny-path <remote-prefix>
--max-download-size <size>
--max-upload-size <size>
--allow-public-shares true|false
--allow-content-search true|false
--allow-search-snippets true|false
--require-dry-run-for-writes true|false
--require-yes-for-sensitive true|false
```

Command families:

```text
auth
profiles
server
files
shares
calendar
contacts
notes
deck
activity
dav
ocs
logs
commands
config
index
update
```

### 20.2 Policy object

Expected policy shape:

```json
{
  "profile": "client-a",
  "agent_mode": "safe",
  "read_only": false,
  "allowed_commands": null,
  "denied_commands": ["profiles", "auth"],
  "allowed_paths": ["/Shared/Client A/"],
  "denied_paths": ["/Personal/", "/Finance/Payroll/"],
  "max_download_size": "100MiB",
  "max_upload_size": "100MiB",
  "allow_public_shares": false,
  "allow_content_search": true,
  "allow_search_snippets": false,
  "require_dry_run_for_writes": false,
  "require_yes_for_sensitive": true
}
```

`null` for `allowed_commands` means all command families are allowed unless
denied. Empty arrays mean no explicit entries.

### 20.3 Agent modes

Agent mode is per profile. It may be activated by `--agent`,
`NEXTCLOUD_CLI_AGENT_MODE=true`, or profile policy default.

Modes:

| Mode | Behavior |
| --- | --- |
| `off` | Human defaults. JSON remains default, but no extra agent restrictions. |
| `safe` | JSON-only, lower list limits, no public shares without `--yes`, no profile/auth mutation without `--yes`, explicit profile required when ambiguous. |
| `strict` | Read-only by default, no public shares, no raw DAV/OCS writes, lower transfer limits, writes require explicit policy allow-list. |

Default for new profiles:

```text
agent_mode = safe
```

Human users can relax it, but generated setup skill guidance should keep `safe`
for agent-managed profiles.

### 20.4 Policy enforcement order

Before executing a command, enforce policy in this order:

1. Resolve effective profile.
2. Resolve effective agent mode.
3. Classify command safety using command metadata.
4. Check command family allow/deny policy.
5. Check remote path allow/deny policy where applicable.
6. Check transfer size limits where applicable.
7. Check destructive/sensitive confirmation requirements.
8. Only then build and send the HTTP request.

Policy failures return structured errors and must not make network requests.

Example:

```json
{
  "error": {
    "code": "policy_denied",
    "message": "Profile 'client-a' does not allow public share creation.",
    "hint": "Change the profile policy or choose a different profile.",
    "source": "cli",
    "details": {
      "profile": "client-a",
      "command": "shares.create",
      "policy": "allow_public_shares"
    }
  }
}
```

### 20.5 Path policy

Path allow/deny checks apply to remote Nextcloud paths, not local filesystem
paths. They must run after remote path normalization and before URL encoding.

Rules:

- deny rules win over allow rules
- path prefixes must match complete path segments
- `/Client` must not match `/Clientele`
- root `/` allow means all paths unless denied
- raw DAV requests must still be checked when the target maps to a user file path

### 20.6 Public share policy

Because public share creation is sensitive, default agent-safe behavior is:

```text
allow_public_shares = false
require_yes_for_sensitive = true
```

If public shares are allowed, non-interactive contexts still require `--yes`.
Interactive contexts may prompt. Agent setup docs must tell agents to avoid
creating public shares unless the user explicitly asked for it.

### 20.7 Policy completion gate

Per-profile policy is complete only when:

- policy show/set/reset commands exist
- new profiles get safe agent defaults
- `--agent` and `NEXTCLOUD_CLI_AGENT_MODE` activate agent behavior
- command safety classes drive policy checks
- path allow/deny checks are segment-safe
- public share creation requires explicit confirmation in non-interactive contexts
- tests cover read-only mode, command deny, path deny, size limits, public share
  denial, content-search policy, snippet suppression, and agent-mode defaults

## 21. Server Capability and Version Detection Spec

The CLI must understand the server it is talking to before it assumes feature
support. Nextcloud deployments vary by server version, enabled apps, app
versions, provider configuration, and organization policy.

Capability detection is part of the MVP because agents need clear answers to
questions like "can I use Deck here?" and "does this server support the file
search path I am about to call?"

### 21.1 Capability commands

Public commands:

```bash
nextcloud-cli server status
nextcloud-cli server capabilities [--refresh]
nextcloud-cli server apps [--refresh]
nextcloud-cli server doctor [--refresh]
```

`server status` returns basic reachability and version metadata.

`server capabilities` returns normalized capabilities from the OCS capabilities
endpoint and CLI-derived feature support.

`server apps` returns known app availability for the feature surfaces this CLI
cares about.

`server doctor` runs a read-only compatibility check for the selected profile.

### 21.2 Backing APIs

Primary built-in endpoints:

- `GET /status.php`
- `GET /ocs/v2.php/cloud/capabilities`

App availability may be inferred from:

- capabilities payloads where present
- known app endpoints returning 200, 401, 403, or 404
- DAV discovery for files, calendars, and address books
- optional app API probes for Notes, Deck, and Activity

The CLI must not require admin-only provisioning APIs to detect user-level feature
support.

### 21.3 Capability output

Expected `server capabilities` output:

```json
{
  "profile": "personal",
  "server": "https://cloud.example.com",
  "version": {
    "raw": "31.0.0.0",
    "major": 31,
    "minor": 0,
    "micro": 0,
    "string": "Nextcloud Hub"
  },
  "features": {
    "files": {
      "available": true,
      "backend": "webdav"
    },
    "file_search": {
      "available": true,
      "backend": "dav_search"
    },
    "shares": {
      "available": true,
      "backend": "ocs_files_sharing"
    },
    "calendar": {
      "available": true,
      "backend": "caldav"
    },
    "contacts": {
      "available": true,
      "backend": "carddav"
    },
    "notes": {
      "available": false,
      "backend": "notes_api_v1",
      "reason": "app_unavailable"
    },
    "deck": {
      "available": false,
      "backend": "deck_api_v1",
      "reason": "app_unavailable"
    },
    "activity": {
      "available": true,
      "backend": "activity_ocs_v2"
    }
  },
  "checked_at": "2026-04-10T15:04:05Z",
  "cache": {
    "used": false,
    "ttl_seconds": 3600
  }
}
```

### 21.4 Capability cache

Capabilities may be cached per profile to avoid probing every command.

Requirements:

- Default TTL is 1 hour.
- `--refresh` bypasses cache and rewrites it.
- Auth changes invalidate the cache for that profile.
- Server URL changes invalidate the cache.
- Cache contents must not include secrets.
- Cache is advisory. Commands must still handle server errors honestly.

### 21.5 Compatibility behavior

If a feature is unavailable, the command should fail before making a known-bad API
call when cached capability data is fresh. If no cache exists, the command may
probe or attempt the request and map failures into structured errors.

Example:

```json
{
  "error": {
    "code": "feature_unavailable",
    "message": "Deck is not available for profile 'client-a'.",
    "hint": "Run `nextcloud-cli server apps --profile client-a --refresh` or enable the Deck app on that server.",
    "source": "cli",
    "details": {
      "profile": "client-a",
      "server": "https://cloud.client-a.org",
      "feature": "deck"
    }
  }
}
```

### 21.6 Capability completion gate

Capability detection is complete only when:

- status, capabilities, apps, and doctor commands exist
- capabilities are cached per profile
- cache refresh works
- optional app availability is represented consistently
- auth changes invalidate relevant cache entries
- tests cover supported, unsupported, stale cache, refresh, and malformed server
  responses
- real-server smoke report includes capability snapshot

## 22. Large File Upload and Transfer Spec

The CLI must handle both small and large files deliberately. A simple WebDAV
`PUT` is fine for small files, but large uploads need resumable/chunked behavior
so agents and scripts do not restart a multi-gigabyte upload from byte zero after
a network hiccup.

### 22.1 Upload modes

`files upload` supports these modes:

```bash
nextcloud-cli files upload <local> <remote> --upload-mode auto
nextcloud-cli files upload <local> <remote> --upload-mode put
nextcloud-cli files upload <local> <remote> --upload-mode chunked
```

Default:

```text
--upload-mode auto
```

Auto mode chooses:

- simple WebDAV `PUT` for files below the chunk threshold
- chunked WebDAV upload for files at or above the chunk threshold

### 22.2 Chunk threshold and chunk size

Defaults:

- chunk threshold: `100 MiB`
- chunk size: `10 MiB`

Configurable flags:

```bash
--chunk-threshold <size>
--chunk-size <size>
```

Sizes accept explicit units:

```text
10MiB
100MiB
1GiB
```

Decimal and binary unit parsing must be documented. If implementation supports
only binary units initially, reject ambiguous inputs clearly.

### 22.3 Chunked upload protocol

Chunked uploads should use Nextcloud's WebDAV upload collection pattern:

```text
/remote.php/dav/uploads/{username}/{upload-id}/
```

Expected high-level flow:

1. Create a unique upload id.
2. Upload numbered chunks into the upload collection.
3. Assemble the final file with `MOVE` from the upload collection `.file` target
   to the destination under `/remote.php/dav/files/{username}/`.
4. Verify the final file with a follow-up `PROPFIND`.
5. Clean up abandoned chunks where possible.

The exact protocol details must be verified against the local server source and
current Nextcloud developer documentation during implementation. The product
decision is fixed: large upload support must be resumable/chunked, not simple
`PUT` only.

### 22.4 Resume behavior

The MVP should support safe retry within a single process. Persistent resume
after process exit is preferred but may be implemented after the basic chunked
path if explicitly tracked.

Minimum MVP behavior:

- retry failed chunk uploads according to the HTTP retry policy
- fail with enough metadata to diagnose the failed chunk
- avoid reporting success unless final assembly and verification succeed
- avoid leaving credentials or file bytes in logs

Preferred behavior:

- persist upload session metadata under cache directory
- resume an interrupted upload when local file size, mtime, and destination match
- expose `--no-resume` to force a clean upload

### 22.5 Transfer progress

Progress belongs on stderr only and only when stderr is a TTY.

JSON stdout must remain parseable.

`--quiet` disables progress.

`--verbose` may emit transfer diagnostics to stderr, still secret-free.

### 22.6 Download behavior

`files download` should support large downloads with streaming writes.

Requirements:

- never buffer entire file in memory
- write to a temp file first, then atomically rename when complete
- refuse overwrite unless `--overwrite`
- verify byte count when content length is available
- clean up partial temp files unless `--keep-partial` is added in the future

### 22.7 Transfer completion gate

Large transfer support is complete only when:

- simple upload works below threshold
- chunked upload works above threshold
- threshold and chunk size are configurable
- progress never corrupts JSON stdout
- large download streams to disk
- interrupted or failed transfers do not report success
- tests cover threshold selection, chunk naming, retries, final assembly, local
  overwrite refusal, and binary integrity
- smoke test uploads and downloads a file large enough to exercise the selected
  large-file path, unless explicitly skipped with rationale


## 23. Search Backend and Content Search Spec

Content search is important enough to define as its own feature spec. The MVP
must provide honest search behavior rather than pretending every Nextcloud server
can search file contents the same way.

The product goal is:

```text
files search should search filenames and metadata everywhere, and file contents
when either the selected server exposes a supported content-search backend or the
user has explicitly built a local client-side index.
```

There are two different content-search strategies:

1. Remote/server-backed content search, using Nextcloud apps or providers that
   already index content on the server.
2. Local/client-side indexing, using a local SQLite index and, later, a
   markdown-centric knowledge graph built from files the user has explicitly
   allowed the CLI to crawl.

These strategies must not be blurred together. Remote search depends on server
capabilities. Local search depends on client-side indexing, local storage policy,
profile policy, and user consent.

### 23.1 Search modes

`files search` supports search modes:

```bash
nextcloud-cli files search <query> --search-mode auto
nextcloud-cli files search <query> --search-mode name
nextcloud-cli files search <query> --search-mode content
nextcloud-cli files search <query> --search-mode unified
nextcloud-cli files search <query> --search-mode local
```

Default:

```text
--search-mode auto
```

Mode behavior:

| Mode | Behavior |
| --- | --- |
| `name` | Search file names and file metadata through DAV search. |
| `content` | Search indexed file contents through a supported remote full-text backend. |
| `unified` | Search through Nextcloud Unified Search providers. |
| `local` | Search a local client-side index for the selected profile. |
| `auto` | Use the best available backend and report which backend was used. |

### 23.2 Search backend priority

In `auto` mode, use this preference order:

1. Local client-side index when it exists, is fresh enough, and policy allows it.
2. Supported remote content-search backend when available and policy allows it.
3. Unified Search files provider when available.
4. DAV `SEARCH` for filename and metadata search.

The response must always report the backend:

```json
{
  "query": "contract",
  "search_mode": "auto",
  "backend": "fulltextsearch",
  "content_search": true,
  "files": [],
  "count": 0
}
```

### 23.3 Remote content search behavior

Remote content search may use FullTextSearch app APIs or Unified Search providers,
depending on server capability. Implementation must verify exact endpoints
against the selected supported app versions before coding.

Remote content search is server-side. It depends on apps, indexers, and providers
configured on the Nextcloud instance. The CLI must not pretend remote content
search exists just because a user wants it.

The spec requires remote content search support when a known supported backend is
available. It does not require the CLI to install or administer the server-side
indexer.

If the user asks for `--search-mode content` and no supported remote content
backend is available, return:

```json
{
  "error": {
    "code": "content_search_unavailable",
    "message": "Content search is not available for profile 'personal'.",
    "hint": "Use `--search-mode name` or enable a supported Nextcloud full-text search backend.",
    "source": "files"
  }
}
```

### 23.4 Search result object

Search results should extend the common file object with search metadata:

```json
{
  "name": "contract.pdf",
  "path": "/Documents/contract.pdf",
  "type": "file",
  "mime_type": "application/pdf",
  "score": 0.91,
  "matched_fields": ["content"],
  "snippet": "...payment terms...",
  "backend": "fulltextsearch"
}
```

`snippet` may be `null` if the backend does not provide safe snippets.

### 23.5 Privacy and content search

Content search can reveal sensitive text snippets. Policy must control it.

Profile policy should eventually support:

```bash
--allow-content-search true|false
--allow-search-snippets true|false
```

MVP default for agent-safe mode:

```text
allow_content_search = true
allow_search_snippets = false
```

That means agents may find files by content when supported, but the CLI should
not expose content snippets unless the profile allows them.

### 23.6 Local index and markdown knowledge graph

A client-side local index is likely the better long-term content-search path for
agent use. It gives users a search layer that does not depend on server-side
FullTextSearch configuration and can become more agent-native over time.

Local indexing must be opt-in. The CLI must never crawl or index file contents
implicitly during normal `files search`, `files list`, or setup flows. A user or
agent must run an explicit `index build` or `index update` command, and profile
policy must allow the selected crawl roots.

Local index goals:

- per-profile SQLite storage
- explicit user-controlled crawl roots
- incremental indexing using WebDAV metadata, ETags, file ids, and modified times
- filename, path, MIME type, size, and modified-time indexing
- text extraction for markdown, plaintext, and other safe text formats first
- optional content extraction for PDFs and office documents later
- snippet suppression by default in agent-safe mode
- local index status visible to users and agents
- no indexing of denied paths
- no indexing above profile size limits
- no hidden background daemon requirement for MVP
- explicit consent before content extraction
- profile-policy enforcement before every indexed download or text extraction

Future local index commands:

```bash
nextcloud-cli index status [--profile <profile>]
nextcloud-cli index build [--path <remote-path>] [--profile <profile>]
nextcloud-cli index update [--profile <profile>]
nextcloud-cli index clear [--profile <profile>] [--yes]
```

`files search --search-mode local` uses this index. If the index has not been
built, return:

```json
{
  "error": {
    "code": "local_index_unavailable",
    "message": "No local search index exists for profile 'personal'.",
    "hint": "Run `nextcloud-cli index build --profile personal` or use `--search-mode name`.",
    "source": "files"
  }
}
```

The markdown-centric knowledge graph is a later layer on top of the local index,
not the first implementation step. It should focus on explicit markdown vaults,
notes, links, headings, tags, backlinks, and references. This should be written as
its own implementation spec before coding.

Local index is not required for the first issue-complete MVP unless explicitly
pulled into scope. The MVP must still avoid blocking it architecturally.

### 23.7 Search completion gate

Content search is complete only when:

- `files search` reports backend and search mode
- name/metadata search works through DAV search
- remote content search works against at least one supported backend or is explicitly
  marked unavailable with a tracked implementation blocker
- local search returns `local_index_unavailable` when no index exists
- unsupported remote content search returns `content_search_unavailable`
- snippets are controlled by profile policy
- tests cover name search, remote content search available, remote content search
  unavailable, local index unavailable, unified fallback, backend reporting, and
  snippet suppression
- real-server smoke tests record which search backend was used

## 24. Pagination, Limits, and Streaming Spec

Every list-like command must have an explicit pagination policy. Agents need to
know whether they are seeing all results, a limited page, or a stream.

### 24.1 Global pagination flags

List-like commands should support these flags unless the backing API makes them
impossible:

```bash
--limit <n>
--cursor <cursor>
--page-size <n>
--all
--stream
```

Meanings:

- `--limit`: maximum items returned to the caller
- `--cursor`: resume from a previous cursor when supported
- `--page-size`: per-request page size where supported
- `--all`: fetch all pages until exhaustion or safety cap
- `--stream`: emit newline-delimited JSON items as they arrive

### 24.2 Default limits

Default limits:

| Command family | Default limit |
| --- | --- |
| files search | 25 |
| shares list | 100 |
| calendar events | no item limit within requested range |
| contacts search | 25 |
| notes list | 100 |
| deck boards | 100 |
| deck cards | 500 |
| activity recent | 20 |

Commands must document whether the limit is enforced client-side, server-side, or
both.

### 24.3 Pagination output

Paginated JSON output should include pagination metadata:

```json
{
  "items": [],
  "count": 0,
  "pagination": {
    "limit": 100,
    "page_size": 100,
    "next_cursor": null,
    "complete": true
  }
}
```

Domain-specific collection keys are allowed, but pagination metadata should be
consistent.

### 24.4 `--all` safety

`--all` must have a safety cap to prevent an agent from accidentally pulling an
entire organization into context.

Default safety cap:

```text
10,000 items
```

If the cap is hit, return success with `complete: false` and a warning field, not
a silent truncation.

### 24.5 Streaming output

`--stream` emits NDJSON, one item per line. It is only valid with `--format json`.

Example:

```jsonl
{"path":"/Documents/a.pdf","type":"file"}
{"path":"/Documents/b.pdf","type":"file"}
```

No summary object should be emitted to stdout in stream mode unless a future
`--stream-summary` flag is added. Diagnostics belong on stderr.

### 24.6 Pagination completion gate

Pagination is complete only when:

- every list-like command documents limit behavior
- defaults are implemented consistently
- `--all` has a safety cap
- truncation is explicit in output metadata
- stream mode never emits non-JSON prose on stdout
- tests cover limit, cursor where supported, all, safety cap, and stream output

## 25. Destructive Action and Confirmation Spec

The CLI must make dangerous actions hard to do accidentally, especially when an
AI agent is operating across multiple Nextcloud profiles.

### 25.1 Destructive or sensitive actions

These actions are destructive or sensitive:

- overwriting a remote file
- overwriting a local file during download
- deleting local profile credentials
- revoking app passwords
- removing profiles
- creating public share links
- creating passwordless public share links
- delete, archive, revoke, or remove operations
- calendar event create/update/delete
- contact create/update/delete
- note create/update/delete
- Deck board/stack/card writes
- future delete/move/copy operations if added

### 25.2 Confirmation model

Non-interactive commands must not hang waiting for confirmation unless the command
is explicitly interactive.

Rules:

- If a TTY is available, the CLI may prompt for confirmation for dangerous
  operations.
- If no TTY is available, dangerous operations must require explicit flags.
- `--yes` confirms the operation.
- `--dry-run` shows what would happen and never mutates remote or local state.
- `--force` is reserved for bypassing local safety checks and must be rarer than
  `--yes`.

### 25.3 Public share safety

`shares create --public` is requested in the issue and must exist, but it is a
sensitive operation.

Requirements:

- Output includes `profile` and `server`.
- Dry-run shows path, share type, permissions, password state, and expiration.
- If server policy allows passwordless public links, the command may create one,
  but docs must warn agents not to do this without user intent.
- If the server requires passwords or expiration, surface that OCS error clearly.

### 25.4 Overwrite safety

Download:

- refuse to overwrite a local file unless `--overwrite` is set
- write temp file before final rename

Upload:

- when practical, check whether remote exists before upload
- refuse overwrite unless `--overwrite` is set
- if remote existence cannot be checked, document the fallback behavior in dry-run

### 25.5 Profile removal safety

`profiles remove <profile>` must:

- never remove remote credentials unless `auth logout --revoke` is used
- require `--yes` in non-interactive contexts
- refuse to remove the last profile without an explicit `--yes`
- clear current profile if the removed profile was current

### 25.6 Destructive action completion gate

Destructive safety is complete only when:

- every dangerous command supports `--dry-run`
- every dangerous command has non-interactive safety behavior
- write outputs include `profile` and `server`
- overwrite tests cover local and remote conflicts
- public share tests cover dry-run and password redaction
- profile removal tests cover current profile cleanup and last-profile behavior

## 26. Audit Logging and Local Diagnostics Spec

The CLI should provide local, secret-free audit logs so humans and agents can
understand what happened, especially across multiple organizations.

### 26.1 Logging goals

- help debug failures
- provide an audit trail of agent actions
- avoid leaking secrets or file contents
- keep stdout clean for JSON results
- allow users to disable or delete logs

### 26.2 Log destinations

Default behavior:

- no persistent request log unless enabled
- stderr diagnostics for current command
- optional JSONL log file when `NEXTCLOUD_CLI_LOG_FILE` is set or config enables it

Suggested log path when enabled:

```text
~/.local/state/nextcloud-cli/logs/nextcloud-cli-YYYY-MM-DD.jsonl
```

Use platform-appropriate state directories.

### 26.3 Audit event shape

Audit events should be JSON lines:

```json
{
  "timestamp": "2026-04-10T15:04:05Z",
  "event": "command.executed",
  "profile": "client-a",
  "server": "https://cloud.client-a.org",
  "command": "shares.create",
  "dry_run": false,
  "request": {
    "method": "POST",
    "path": "/ocs/v2.php/apps/files_sharing/api/v1/shares"
  },
  "result": {
    "status": 200,
    "ok": true
  }
}
```

Logs must not include:

- app passwords
- auth headers
- full file contents
- note contents
- contact contents beyond high-level counts
- calendar descriptions
- public share passwords

### 26.4 Agent audit requirements

Write commands should log enough to answer:

- which profile was used?
- which server was used?
- what command family ran?
- was it a dry-run?
- did it succeed?
- what remote path or object id was affected?

For sensitive objects, prefer identifiers and paths over full payloads.

### 26.5 Log controls

Commands:

```bash
nextcloud-cli logs path
nextcloud-cli logs tail [--limit <n>]
nextcloud-cli logs clear [--yes]
```

These commands are optional for MVP if logging can be controlled through config
and environment variables, but the product should reserve the namespace.

### 26.6 Audit completion gate

Audit logging is complete only when:

- logging is structured JSONL when enabled
- secrets are redacted by tests
- write commands produce audit events
- dry-run commands are marked as dry-run
- users can find the log path
- logs never corrupt stdout command JSON

## 27. Machine-readable Command Metadata Spec

The CLI should expose its command surface as machine-readable metadata. This lets
agents, docs, completions, and skills stay aligned with the implemented command
contract.

### 27.1 Metadata command

Public command:

```bash
nextcloud-cli commands schema [--format json]
```

Optional future commands:

```bash
nextcloud-cli commands list
nextcloud-cli commands completions <shell>
```

### 27.2 Metadata shape

Expected schema shape:

```json
{
  "version": "0.1.0",
  "binary": "nextcloud-cli",
  "commands": [
    {
      "name": "files.list",
      "usage": "nextcloud-cli files list <path>",
      "summary": "List files in a folder.",
      "category": "files",
      "safety": "read",
      "default_format": "json",
      "arguments": [
        {
          "name": "path",
          "required": true,
          "type": "string"
        }
      ],
      "flags": [
        {
          "name": "--limit",
          "type": "integer",
          "required": false
        }
      ],
      "output_schema_ref": "#/schemas/FileListOutput",
      "examples": [
        "nextcloud-cli files list /"
      ]
    }
  ],
  "schemas": {}
}
```

### 27.3 Safety classification

Every command must have a safety class:

| Class | Meaning |
| --- | --- |
| `read` | Reads remote or local state only. |
| `write` | Mutates remote or local state. |
| `destructive` | Deletes, overwrites, revokes, or removes access. |
| `sensitive` | May expose or create sensitive access, such as public shares. |

Agents should use safety classes to decide when to ask users before proceeding.

### 27.4 Skill generation

The setup skill may be handwritten. Basic feature skills for files, shares,
calendar, contacts, notes, deck, and activity should ship early and continue to
be refined over time. Future versions may generate parts of these skills from
command metadata plus curated guidance.

Generated skill inputs:

- command schema
- examples
- safety class
- output schemas
- common errors

Generated skills must still be reviewed by a human before release.

### 27.5 Metadata completion gate

Command metadata is complete only when:

- every public command appears in `commands schema`
- every command has safety class
- every command has usage and examples
- output schema refs exist for stable JSON outputs
- docs or skills can be generated from the metadata without hand-discovering
  command flags
- tests snapshot the metadata and catch accidental command contract drift

## 28. Fixtures and Test Data Policy

Fixtures are product infrastructure. They define what upstream responses we
believe in and protect the CLI from drifting when parsers are refactored.

### 28.1 Fixture sources

Allowed fixture sources:

1. Minimal hand-authored fixtures for parser unit tests.
2. Official documentation examples.
3. Sanitized responses from a real smoke server.
4. Local Nextcloud server source examples or tests.
5. Optional app documentation examples.

Every fixture should include a short provenance comment or adjacent `.meta.json`
file.

### 28.2 Fixture layout

Recommended layout:

```text
tests/fixtures/
  webdav/
    propfind-folder.xml
    search-results.xml
  ocs/
    capabilities.json
    share-list.json
    share-create.json
    error.json
  caldav/
    single-event.ics
    all-day-event.ics
    recurring-event.ics
  carddav/
    basic-contact.vcf
    multi-contact.vcf
  notes/
    list.json
  deck/
    boards.json
    stacks.json
  activity/
    recent.json
```

### 28.3 Sanitization requirements

Real-server fixtures must be scrubbed before commit.

Remove or replace:

- real server domains unless intentionally public test domains
- usernames
- display names
- email addresses
- phone numbers
- file names with personal content
- file ids if they reveal real structure
- share tokens
- note content
- calendar descriptions
- auth headers
- cookies
- app passwords

Use stable fake values like:

```text
https://cloud.example.com
user@example.com
nicholai
client-a
report.pdf
```

### 28.4 Snapshot testing policy

Snapshot tests are useful for output stability, but they must not make harmless
format improvements impossible.

Rules:

- snapshot stable JSON command outputs
- snapshot command metadata
- snapshot error envelopes
- do not snapshot timestamps unless fixed
- do not snapshot random upload ids unless injected
- review snapshot diffs as contract changes

### 28.5 Fixture completion gate

Fixture policy is complete only when:

- fixture directory exists
- provenance is documented
- scrub rules are documented
- parser tests use fixtures
- command output snapshots contain no secrets
- CI includes a lightweight secret scan over fixtures and snapshots

## 29. Security Requirements

### 29.1 Credential storage

- Store app passwords encrypted at rest.
- Use OS keyring by default.
- Provide file backend fallback for headless environments.
- Use restrictive permissions for config directories and key files.
- Never store credentials in plaintext unless the user explicitly exports with
  `--unmasked`.

### 29.2 Redaction

Redact:

- app passwords
- Basic auth headers
- bearer tokens if added later
- exported credential secrets unless `--unmasked`
- URLs containing credentials

### 29.3 Local path safety

- Refuse path traversal in local output paths where relevant.
- Refuse to overwrite local files unless `--overwrite` is set.
- Do not follow unsafe symlinks for credential files.

### 29.4 Remote path safety

- Normalize remote paths.
- Percent-encode by path segment.
- Preserve slashes.
- Treat empty path as `/` only for commands where root is valid.
- Reject paths containing NUL bytes or invalid Unicode.

### 29.5 TLS

- Use system trust roots.
- Do not disable TLS verification by default.
- Support custom CA configuration for self-hosted and enterprise deployments.
- `--insecure` must be explicit, noisy, non-persistent, rejected in agent strict
  mode, and test-covered.

## 30. Config, Cache, and Migration Spec

Configuration must be boring, inspectable, and migratable. The CLI will carry
profiles, policy, capability cache metadata, install method tracking, update
state, and eventually local index metadata. That needs a versioned schema from
the start.

### 30.1 Config commands

Public commands:

```bash
nextcloud-cli config path
nextcloud-cli config show [--profile <profile>]
nextcloud-cli config doctor
nextcloud-cli config migrate [--dry-run]
```

Behavior:

- `config path` prints the active config directory and important file paths.
- `config show` prints redacted config by default.
- `config doctor` validates file permissions, schema version, readable profiles,
  credential backend availability, cache shape, and install-method metadata.
- `config migrate --dry-run` shows pending migrations without writing.
- Config commands must never print secrets.

### 30.2 Config layout

Recommended layout:

```text
<config-dir>/
  config.json
  profiles.json
  policy.json
  install.json
  cache/
    capabilities/<profile>.json
    update/latest.json
  logs/
    nextcloud-cli.jsonl
```

Local index data should live under a data directory rather than the config
directory:

```text
<data-dir>/
  indexes/<profile>/index.sqlite
```

Use platform-appropriate config and data directories through the Rust `dirs` or
similar crate. Respect `NEXTCLOUD_CLI_CONFIG_DIR` for config overrides.

### 30.3 Schema versioning

Every persisted JSON config file must include a schema version:

```json
{
  "schema_version": 1
}
```

Rules:

- Unknown newer schema versions fail clearly with `config_version_unsupported`.
- Known older schema versions migrate through explicit migration functions.
- Migrations must be idempotent.
- Failed migrations must leave the original file recoverable.
- Migration tests must use real fixture files.

### 30.4 Install method tracking

`install.json` should record enough information for update behavior:

```json
{
  "schema_version": 1,
  "install_method": "github-release",
  "binary_path": "/usr/local/bin/nextcloud-cli",
  "alias_path": "/usr/local/bin/nxc",
  "installed_version": "0.1.0",
  "installed_at": "2026-04-10T15:04:05Z"
}
```

The npm wrapper and curl installer should write or update this metadata when
possible. Cargo and package-manager installs may report `unknown` or a detected
package-manager method.

### 30.5 Config completion gate

Config support is complete only when:

- config path/show/doctor/migrate commands exist
- persisted config files are schema-versioned
- migrations are idempotent and test-covered
- config output is redacted by default
- file permissions are checked on Unix-like systems
- install method tracking supports update check/apply behavior

## 31. Network, Proxy, TLS, and Polite Concurrency Spec

The CLI must work against home servers, enterprise Nextcloud instances, reverse
proxies, and self-signed deployments without making unsafe behavior easy by
accident.

### 31.1 Proxy behavior

Requirements:

- Honor `HTTPS_PROXY`, `HTTP_PROXY`, and `NO_PROXY` when supported by the HTTP
  stack.
- Document proxy behavior in README and troubleshooting docs.
- Never log proxy credentials.
- Include proxy configuration in `config doctor` without exposing secrets.

### 31.2 TLS and custom CA behavior

Requirements:

- Use system trust roots by default.
- Support `--ca-bundle <path>` and `NEXTCLOUD_CLI_CA_BUNDLE` for self-hosted and
  enterprise deployments.
- `--insecure` may exist, but it must be explicit, noisy, non-persistent, and
  test-covered.
- `--insecure` must be rejected in agent strict mode.
- Error messages for certificate failures should suggest custom CA configuration
  before suggesting `--insecure`.

### 31.3 Polite concurrency and retries

Bulk operations must have a concurrency policy.

Defaults:

```text
max_concurrency = 4
max_retries = 3
retry_backoff = exponential_with_jitter
retry_after_cap = 60s
```

Requirements:

- Honor `Retry-After` for 429 and 503 responses.
- Do not retry unsafe non-idempotent writes unless the command has explicit retry
  semantics.
- Local indexing must use bounded concurrency.
- Large uploads must not create unbounded parallel chunk uploads by default.
- Provide a documented way to lower concurrency for fragile servers.

### 31.4 Network completion gate

Network support is complete only when:

- proxy environment variables are honored or documented as unsupported with a
  clear implementation blocker
- custom CA bundle support works in tests
- `--insecure` is noisy, non-persistent, and blocked by strict agent mode
- retry and concurrency defaults are implemented centrally
- tests cover `Retry-After`, retry cap, proxy credential redaction, custom CA
  config, and strict-mode insecure rejection

## 32. Validation Items

Before MVP can be considered complete, validate all of the following:

### 32.1 Product validation

- Every command requested in issue #59417 exists.
- Every command returns JSON by default.
- Every command has at least one documented example.
- Every command has a defined output shape.
- Every command has defined failure behavior.

### 32.2 Auth validation

- Login Flow v2 works against a real server.
- Manual app-password auth works against a real server.
- Revoked or invalid credentials fail clearly.
- Missing credentials produce an actionable hint.
- Credential export masking works.
- Secrets are absent from logs and error JSON.

### 32.3 Files validation

- Root folder listing works.
- Nested folder listing works.
- File stat works.
- Folder creation works.
- Upload works.
- Download works.
- Search works.
- Path encoding handles spaces and special characters.
- Missing file and permission errors are structured.

### 32.4 Shares validation

- Share listing works.
- Public link creation works.
- Public link revoke/delete works.
- Password, expiration, and permission fields are normalized.
- OCS errors are mapped correctly.

### 32.5 Calendar validation

- Calendar discovery works.
- Today query works.
- Seven-day range query works.
- All-day events are represented correctly.
- Timezone behavior is documented and tested.

### 32.6 Contacts validation

- Address book discovery works.
- Contact search works.
- Empty search results are successful.
- vCard parsing handles common fields.

### 32.7 Optional app validation

- Notes success path works in mocks.
- Deck success path works in mocks.
- Activity success path works in mocks.
- Missing optional app behavior is structured.
- Real-server smoke tests record enabled or unavailable state per optional app.

### 32.8 Write operation validation

- Calendar create/update/delete works against mocks and optional write smoke.
- Contact create/update/delete works against mocks and optional write smoke.
- Notes create/update/delete works when Notes is enabled or returns
  `app_unavailable`.
- Deck board/stack/card writes work when Deck is enabled or return
  `app_unavailable`.
- Every write command supports `--dry-run`.
- Every destructive write requires confirmation in non-interactive contexts.
- Write outputs include selected profile and server.
- ETag or conflict behavior is tested where the upstream API supports it.

### 32.9 Agent usability validation

- No command emits prose on stdout in JSON mode.
- No command emits ASCII art in JSON mode or agent mode.
- Empty results are valid JSON.
- Errors are valid JSON.
- `jq` can parse all command outputs.
- Examples are copy-pasteable.


### 32.10 Multi-account validation

- Multiple stored profiles can coexist.
- The same server can be configured with multiple users.
- Different servers can be configured with different organizations.
- Ambiguous profile selection fails with `profile_required`.
- `--profile` and `NEXTCLOUD_CLI_PROFILE` precedence works.
- Write outputs include selected profile and server.

### 32.11 Capability validation

- Server status works against a real server.
- OCS capabilities are fetched and normalized.
- Optional app availability is detected or probed.
- Capability cache is used, refreshed, and invalidated correctly.
- Feature-unavailable errors are structured.

### 32.12 Transfer validation

- Simple upload path works.
- Chunked upload path works or has an explicit implementation blocker recorded.
- Large download streams to disk.
- Progress appears only on stderr.
- Overwrite safety works for local and remote conflicts.

### 32.13 Pagination validation

- Every list command documents and enforces default limits.
- `--limit` works consistently.
- `--all` has an explicit safety cap.
- `--stream` emits only NDJSON on stdout.
- Truncation is visible in output metadata.

### 32.14 Destructive action validation

- Sensitive and destructive commands support `--dry-run`.
- Non-interactive destructive commands require explicit confirmation flags.
- Public share creation includes profile and server in output.
- Public share deletion requires confirmation in non-interactive contexts.
- Passwords and tokens are redacted.

### 32.15 Audit and metadata validation

- Audit logging is structured when enabled.
- Audit logs do not include secrets or content payloads.
- `commands schema` includes every public command.
- Command metadata includes safety class and examples.
- Metadata snapshots catch command-contract drift.

### 32.16 Fixture validation

- Fixtures have documented provenance.
- Real-server fixtures are sanitized.
- Parser tests use fixtures.
- Fixture and snapshot secret scan passes.

### 32.17 Config and migration validation

- `config path`, `config show`, `config doctor`, and `config migrate --dry-run`
  work without exposing secrets.
- Config files include schema versions.
- Older fixture configs migrate cleanly.
- Unknown newer schema versions fail with `config_version_unsupported`.
- Install method tracking supports update behavior.

### 32.18 Network, proxy, TLS, and concurrency validation

- Proxy environment variables are honored or explicitly documented as unsupported
  with a tracked blocker.
- Proxy credentials are redacted from logs and diagnostics.
- Custom CA bundle configuration works.
- `--insecure` is noisy, non-persistent, and rejected in agent strict mode.
- Retry policy honors `Retry-After` and stops at the retry cap.
- Bulk operations and indexing use bounded concurrency.

### 32.19 Per-profile policy validation

- New profiles receive safe agent defaults.
- `profiles policy show`, `profiles policy set`, and `profiles policy reset`
  work against stored profiles.
- Policy denial happens before network requests.
- Path policies match complete path segments.
- Content search and snippet policy are enforced.
- Public share creation requires explicit confirmation in non-interactive agent
  contexts when policy allows it.

### 32.20 Search and content-search validation

- `files search --search-mode name` works through DAV search.
- `files search --search-mode auto` reports the backend used.
- `files search --search-mode content` succeeds against at least one supported
  content-search backend or returns `content_search_unavailable`.
- `files search --search-mode local` returns `local_index_unavailable` until the
  user explicitly builds an index.
- Snippets are suppressed by default in agent-safe mode.
- Real-server smoke reports record whether name search, unified search, or
  content search was used.

### 32.21 Compatibility validation

- MVP behavior is validated against the latest supported Nextcloud server.
- Older-server testing can be added after MVP without changing command contracts.
- `docs/COMPATIBILITY.md` records tested server versions, enabled apps, and known
  feature gaps.
- Unsupported server features produce structured errors.

### 32.22 Distribution and update validation

- GitHub Release archives exist for all supported targets.
- Each archive has a checksum.
- npm installation downloads and verifies the matching release artifact.
- curl installer downloads and verifies the matching release artifact.
- `run.js` reinstalls the binary if missing, matching the reference CLI lesson.
- `update check` detects current and latest versions without mutating state.
- `update apply --yes` updates direct GitHub Release installs or prints a
  package-manager fallback for npm, Cargo, Homebrew, and unknown installs.

### 32.23 Agent setup skill validation

- Setup skill can choose an install method without guessing.
- Setup skill validates `nextcloud-cli --version` and read-only auth/file access.
- Setup skill keeps profile policy in safe agent mode by default.
- Setup skill never asks for or stores a primary account password.

### 32.24 ASCII art and terminal identity validation

- Human-facing help or first-run flows include terminal art when stdout is a TTY.
- JSON, NDJSON, redirected stdout, and agent mode never include art.
- `--no-art` and `NEXTCLOUD_CLI_NO_ART=1` suppress art.
- Art assets fit within 80 columns or have documented fallbacks.
- Snapshot tests cover at least one art-enabled human output and one art-disabled
  machine output.

## 33. Test Suite Spec

### 33.1 Unit tests

Required unit test groups:

- config path resolution
- profile selection
- credential precedence
- keyring/file backend selection
- secret redaction
- server URL normalization
- proxy and custom CA configuration redaction
- remote path normalization
- path segment encoding
- config schema version parsing
- config migration idempotence
- Login Flow v2 parsing
- OCS envelope parsing
- OCS error mapping
- WebDAV multistatus parsing
- DAV search XML generation
- content-search backend selection
- local index consent and unavailable state
- content-search policy enforcement
- search snippet suppression
- iCalendar parsing
- iCalendar serialization
- CalDAV ETag and If-Match handling
- vCard parsing
- vCard serialization
- CardDAV ETag and If-Match handling
- Notes response parsing
- Notes write payload generation
- Deck response parsing
- Deck write payload generation
- Activity response parsing
- output formatting
- art enable/disable detection
- TTY detection for human output
- exit code mapping
- profile ambiguity and selection precedence
- profile policy parsing and enforcement
- agent mode selection
- capability cache invalidation
- chunk threshold and size parsing
- pagination metadata generation
- destructive action confirmation policy
- audit event redaction
- command metadata generation
- update platform and install-method detection
- update checksum verification
- retry and concurrency policy
- ASCII art width validation
- fixture provenance validation

### 33.2 Mock HTTP integration tests

Required mocked command tests:

- `auth login`
- `auth add`
- `auth status`
- `auth export`
- `auth logout`
- `profiles list`
- `profiles current`
- `profiles use`
- `profiles show`
- `profiles rename`
- `profiles remove`
- `profiles policy show`
- `profiles policy set`
- `profiles policy reset`
- `server status`
- `server capabilities`
- `server apps`
- `server doctor`
- `files list`
- `files stat`
- `files mkdir`
- `files search`
- `files search --search-mode content`
- `files search --search-mode unified`
- `files download`
- `files upload`
- `shares list`
- `shares create --public`
- `shares delete`
- `shares revoke`
- `calendar events --date today`
- `calendar events --range 7d`
- `calendar create`
- `calendar update`
- `calendar delete`
- `contacts search`
- `contacts create`
- `contacts update`
- `contacts delete`
- `notes list`
- `notes create`
- `notes update`
- `notes delete`
- `deck boards`
- `deck boards create`
- `deck stacks create`
- `deck cards --board`
- `deck cards create`
- `deck cards update`
- `deck cards move`
- `deck cards archive`
- `deck cards delete`
- `activity recent`
- `dav request`
- `ocs request`
- `commands schema`
- `config path`
- `config show`
- `config doctor`
- `config migrate --dry-run`
- `update check`
- `update apply --yes`
- `--help` art-enabled TTY output
- `--help --no-art` plain output
- `logs path` if log commands are implemented
- `logs tail` if log commands are implemented
- `logs clear` if log commands are implemented

Each mocked command test must assert:

- request method
- request URL
- required headers
- auth presence without exposing secret value
- request body where applicable
- stdout JSON shape
- exit code

### 33.3 Fixture tests

Required fixtures:

- WebDAV `207 Multi-Status` folder listing
- WebDAV `207 Multi-Status` file stat
- WebDAV `MKCOL` success and failure responses
- DAV `SEARCH` response
- Unified Search file-provider response
- FullTextSearch OCS response if a supported backend is selected
- OCS success envelope
- OCS failure envelope
- iCalendar single event
- iCalendar all-day event
- iCalendar recurring event
- iCalendar generated event for write tests
- CalDAV conflict response
- vCard basic contact
- vCard multiple emails and phones
- vCard generated contact for write tests
- CardDAV conflict response
- Notes list response
- Notes create/update/delete responses
- Deck boards response
- Deck stacks/cards response
- Deck write responses
- Activity response
- server status response
- capabilities response
- optional app unavailable response
- command metadata schema snapshot
- audit event snapshot
- GitHub Release metadata response for `update check`
- checksum file for update/install tests
- config migration fixtures
- ASCII art assets for human output snapshots

### 33.4 Real-server smoke tests

The smoke suite should be runnable manually with environment variables:

```bash
NEXTCLOUD_URL=https://cloud.example.com \
NEXTCLOUD_USER=nicholai \
NEXTCLOUD_APP_PASSWORD=... \
cargo test --test smoke -- --ignored
```

Write smoke tests must be opt-in through an environment variable such as
`NEXTCLOUD_CLI_SMOKE_WRITES=1`. When disabled, write-capable optional apps should
still be mock-tested and recorded as skipped in the smoke report.

Smoke tests must create a unique test prefix, for example:

```text
/nextcloud-cli-smoke-<timestamp>/
```

Required smoke validations:

1. Auth status succeeds.
2. Server status and capabilities succeed.
3. Test folder can be created with `files mkdir`.
4. Test folder can be inspected with `files stat`.
5. Test file uploads.
6. Test folder lists uploaded file.
7. Test file downloads with matching bytes.
8. File search finds uploaded file and records the search backend.
9. Content search is attempted when a supported backend is detected, or recorded
   as unavailable.
10. Public share can be created for uploaded file when policy and confirmation
    allow it.
11. Public share is deleted during cleanup.
12. Calendar discovery succeeds.
13. Calendar range query succeeds, even if empty.
14. Contacts discovery succeeds.
15. Contacts search succeeds, even if empty.
16. Notes list succeeds or records app unavailable.
17. Deck boards succeeds or records app unavailable.
18. Activity recent succeeds or records app unavailable.
19. Optional write smoke creates, updates, and deletes a calendar event when enabled.
20. Optional write smoke creates, updates, and deletes a contact when enabled.
21. Optional write smoke creates, updates, and deletes a note when Notes is enabled.
22. Optional write smoke creates, updates, and deletes a Deck card when Deck is enabled.
23. Cleanup removes uploaded test artifacts where possible.

Smoke output should be written to a JSON report file:

```text
target/nextcloud-cli-smoke-report.json
```

### 33.5 Release gate tests

Before release:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test --test smoke -- --ignored
cargo test --test release_distribution
```

If smoke tests cannot run in CI, a release candidate must include a manually
attached smoke report from a real Nextcloud server.

## 34. Completion Gates

### 34.1 Command completion gate

A public command is complete only when all of these are true:

- CLI syntax is implemented.
- Help text is implemented.
- JSON output shape matches this spec.
- Error behavior matches this spec.
- Dry-run behavior works where applicable.
- Unit tests cover parsing and normalization.
- Mock HTTP integration tests cover success and failure.
- Example is documented.
- Secrets are redacted in all outputs.

### 34.2 Feature group completion gate

A feature group is complete only when:

- all commands in the group pass the command completion gate
- shared models are documented
- shared client code has unit tests
- at least one real-server smoke path exists or optional-app unavailable state is
  explicitly recorded

Feature groups:

- auth
- files
- shares
- config, cache, and migrations
- network, proxy, TLS, and concurrency
- calendar read and write
- contacts read and write
- notes read and write
- deck read and write
- activity
- raw requests
- profiles and multi-account selection
- server capability detection
- large transfers
- pagination and streaming
- destructive action safety
- audit logging
- command metadata
- fixtures
- per-profile policy and agent mode
- search backend and content-search behavior
- compatibility matrix
- distribution and self-update
- setup agent skill
- ASCII art and terminal identity

### 34.3 MVP completion gate

The MVP is complete only when:

- all feature groups pass their completion gates
- all commands requested in issue #59417 exist
- share creation has a matching revoke/delete path
- file stat and folder creation exist
- safe write operations exist for calendar, contacts, notes, and Deck
- all outputs are valid JSON by default
- all examples in docs run successfully against mocks or smoke server
- optional app failures are structured
- profile policy defaults protect agent-managed profiles
- content-search behavior is honest about backend availability
- self-update exists from the first release
- terminal art exists for human-facing flows without contaminating machine output
- README quickstart matches implemented behavior
- smoke report exists
- release checklist passes

## 35. Implementation Architecture

### 35.1 Workspace layout

Planned structure:

```text
Cargo.toml
art/
  intro.txt
  outro.txt
  install.txt
  auth-success.txt
  first-run.txt
  features.txt
crates/
  nextcloud/
    Cargo.toml
    src/
      lib.rs
      auth.rs
      client.rs
      dav.rs
      webdav.rs
      ocs.rs
      caldav.rs
      carddav.rs
      notes.rs
      deck.rs
      activity.rs
      search.rs
      capabilities.rs
      transfers.rs
      update.rs
      config_schema.rs
      migrations.rs
      network.rs
      error.rs
      models.rs
  nextcloud-cli/
    Cargo.toml
    src/
      main.rs
      commands.rs
      auth_commands.rs
      profile_commands.rs
      policy_commands.rs
      server_commands.rs
      file_commands.rs
      share_commands.rs
      config_commands.rs
      search_commands.rs
      update_commands.rs
      config.rs
      credential_store.rs
      output.rs
      formatter.rs
      dry_run.rs
      command_metadata.rs
      error.rs
      validate.rs
      smoke.rs
skills/
  nextcloud-cli-setup/SKILL.md
  nextcloud-files/SKILL.md
  nextcloud-shares/SKILL.md
  nextcloud-calendar/SKILL.md
  nextcloud-contacts/SKILL.md
  nextcloud-notes/SKILL.md
  nextcloud-deck/SKILL.md
  nextcloud-activity/SKILL.md
npm/
  package.json
  install.js
  run.js
  platform.js
docs/
  SPEC.md
  SMOKE.md
  COMMANDS.md
  COMPATIBILITY.md
  INSTALL.md
  CONFIG.md
  NETWORK.md
```

This spec does not require implementing all files exactly as listed, but any
alternative layout must preserve the same separation:

- reusable Nextcloud API client library
- CLI command and formatting layer
- credential storage layer
- tests and fixtures

### 35.2 Dependencies

Expected Rust dependencies:

- `tokio`
- `reqwest`
- `serde`
- `serde_json`
- `thiserror`
- `anyhow` only at binary boundaries where appropriate
- `clap`
- `dirs`
- `keyring`
- `aes-gcm`
- `zeroize`
- `quick-xml` or equivalent XML parser
- `icalendar` or lower-level iCalendar parser if adequate
- vCard parser crate or small internal parser if crate quality is poor
- `tracing`
- `tracing-subscriber`
- `tempfile` for tests
- HTTP mock crate for integration tests

Dependency choices must be reviewed for maintenance and license compatibility
before implementation.

### 35.3 HTTP client

The shared client must provide:

- connection pooling
- sane connect timeout
- retry for transient network errors and 429
- `Retry-After` handling with a max cap
- user-agent identifying `nextcloud-cli/<version>`
- auth injection
- dry-run request rendering
- secret redaction

### 35.4 API client boundaries

The library crate should expose typed clients:

- `NextcloudClient`
- `WebDavClient`
- `OcsClient`
- `CalDavClient`
- `CardDavClient`
- `NotesClient`
- `DeckClient`
- `ActivityClient`
- `SearchClient`
- `CapabilitiesClient`
- `UpdateClient`
- `ConfigStore`
- `MigrationRunner`

The CLI should not hand-build endpoint URLs except in raw request commands,
installer/update code, and intentionally isolated release-discovery helpers.



## 36. Compatibility and Backwards Compatibility Spec

The MVP supports the latest stable Nextcloud server by default. Older versions
are supported as compatibility work after the MVP exists and can be tested
against real servers.

This is a product decision: build the correct modern client first, then add
backwards compatibility intentionally instead of freezing the MVP around unknown
legacy behavior.

### 36.1 Compatibility target

Initial target:

```text
latest stable Nextcloud server at implementation time
```

Compatibility posture:

- latest stable: supported
- recent supported releases: best effort until tested
- older personal deployments: add support as real test instances expose gaps
- optional app versions: support current documented APIs first

### 36.2 Compatibility matrix

The spec should maintain a compatibility matrix once implementation begins:

```text
docs/COMPATIBILITY.md
```

Minimum matrix fields:

```json
{
  "server_version": "31.0.0",
  "profile": "smoke-latest",
  "files": "pass",
  "file_search": "pass",
  "content_search": "unavailable",
  "shares": "pass",
  "calendar": "pass",
  "contacts": "pass",
  "notes": "app_unavailable",
  "deck": "app_unavailable",
  "activity": "pass",
  "tested_at": "2026-04-10T15:04:05Z"
}
```

### 36.3 Backwards compatibility workflow

When an older server fails:

1. Capture sanitized smoke output.
2. Identify whether the failure is auth, DAV, OCS, optional app, parser, or
   policy behavior.
3. Add a fixture for the older response shape.
4. Add compatibility logic only when it does not weaken modern behavior.
5. Record the server version in the compatibility matrix.

### 36.4 Compatibility completion gate

Compatibility support is complete for a server version only when:

- real-server smoke tests have been run against that version
- failures are documented or fixed
- fixtures cover any divergent response shapes
- compatibility matrix records the result
- unsupported features fail with structured errors, not panics or malformed JSON

## 37. Distribution and Installation Spec

Distribution should feel as close as practical to the Google Workspace CLI in
`references/cli`: users can download a release binary directly, install through
npm as a convenience wrapper, install through Cargo, and eventually install
through package managers like Homebrew. The install path should feel boring,
fast, and trustworthy.

### 37.1 Distribution goals

1. A user can install without building Rust locally.
2. An agent can install without guessing platform-specific archive names.
3. Every downloaded binary has a checksum.
4. Release artifacts are attached to GitHub Releases.
5. npm installation downloads the matching GitHub Release binary.
6. A curl/bash installer downloads the matching GitHub Release binary.
7. Cargo installation remains available for Rust users.
8. Install instructions are copy-pasteable and test-covered where practical.

### 37.2 Primary installation methods

The MVP should document and support these methods:

```bash
# npm convenience install
npm install -g nextcloud-cli

# curl installer
curl -fsSL https://raw.githubusercontent.com/<owner>/<repo>/main/install.sh | sh

# cargo install from crates.io
cargo install nextcloud-cli --locked

# direct GitHub Release download
curl -sLO https://github.com/<owner>/<repo>/releases/download/v<VERSION>/nextcloud-cli-<target>.tar.gz
curl -sLO https://github.com/<owner>/<repo>/releases/download/v<VERSION>/nextcloud-cli-<target>.tar.gz.sha256
shasum -a 256 -c nextcloud-cli-<target>.tar.gz.sha256
tar -xzf nextcloud-cli-<target>.tar.gz
chmod +x nextcloud-cli
sudo mv nextcloud-cli /usr/local/bin/
```

The README should present direct release download as the most deterministic
method, npm as the most convenient method for most agent environments, and Cargo
as the native Rust method.

A curl installer should also be supported because it is convenient for humans,
servers, and agents that do not want npm as the installer layer:

```bash
curl -fsSL https://raw.githubusercontent.com/<owner>/<repo>/main/install.sh | sh
```

The install script must:

- detect OS and architecture
- download the matching GitHub Release archive
- verify SHA256 checksum
- install both `nextcloud-cli` and `nxc`
- print the install path
- avoid requiring root unless installing into a system path
- support an override like `INSTALL_DIR=$HOME/.local/bin`
- fail loudly on checksum mismatch
- avoid printing secrets or environment dumps

### 37.3 Binary names

Release archives must contain:

- `nextcloud-cli`
- `nxc`

On Windows:

- `nextcloud-cli.exe`
- `nxc.exe`

The alias may be a copied binary, symlink, hardlink, or wrapper depending on the
platform packaging constraints. Both commands must report the same version and
must produce identical behavior.

### 37.4 Release targets

The release workflow should build these targets unless a target proves
unreasonable during implementation:

| Target | Archive | Binary |
| --- | --- | --- |
| `aarch64-apple-darwin` | `.tar.gz` | `nextcloud-cli` |
| `x86_64-apple-darwin` | `.tar.gz` | `nextcloud-cli` |
| `aarch64-unknown-linux-gnu` | `.tar.gz` | `nextcloud-cli` |
| `aarch64-unknown-linux-musl` | `.tar.gz` | `nextcloud-cli` |
| `x86_64-unknown-linux-gnu` | `.tar.gz` | `nextcloud-cli` |
| `x86_64-unknown-linux-musl` | `.tar.gz` | `nextcloud-cli` |
| `x86_64-pc-windows-msvc` | `.zip` | `nextcloud-cli.exe` |

This mirrors the platform coverage of `references/cli`.

### 37.5 GitHub Releases

A tag like `v0.1.0` should trigger release publication.

Each release must include:

- platform archives
- `.sha256` files for each archive
- release notes with direct install instructions
- provenance or attestation where available
- changelog summary

Archive naming:

```text
nextcloud-cli-aarch64-apple-darwin.tar.gz
nextcloud-cli-x86_64-apple-darwin.tar.gz
nextcloud-cli-aarch64-unknown-linux-gnu.tar.gz
nextcloud-cli-aarch64-unknown-linux-musl.tar.gz
nextcloud-cli-x86_64-unknown-linux-gnu.tar.gz
nextcloud-cli-x86_64-unknown-linux-musl.tar.gz
nextcloud-cli-x86_64-pc-windows-msvc.zip
```

### 37.6 npm package

The npm package should be a thin installer and runner, matching the feel of
`references/cli/npm`.

Preferred package name:

```text
nextcloud-cli
```

This is a personal project, so if the public npm name is unavailable, use a scope
owned by Nicholai's personal GitHub/npm account and document it explicitly. The
CLI command names must remain `nextcloud-cli` and `nxc` regardless of package
name.

The npm package should contain:

```text
npm/
  package.json
  install.js
  run.js
  platform.js
```

Required behavior:

- `postinstall` runs `install.js`.
- `install.js` detects platform and architecture.
- `install.js` downloads the matching GitHub Release archive for the npm package
  version.
- `install.js` downloads and verifies the `.sha256` file.
- `install.js` extracts the binary into `npm/bin`.
- `run.js` executes the installed binary.
- If the binary is missing, `run.js` attempts reinstall before failing.
- Installer errors are sanitized and must not expose environment secrets.

Supported platform mapping should follow the reference CLI pattern:

- `Darwin` + `x64` -> `x86_64-apple-darwin`
- `Darwin` + `arm64` -> `aarch64-apple-darwin`
- `Linux` + `x64` -> `x86_64-unknown-linux-gnu` or musl when detected
- `Linux` + `arm64` -> `aarch64-unknown-linux-gnu` or musl when detected
- `Windows_NT` + `x64` -> `x86_64-pc-windows-msvc`

The npm wrapper must expose both commands:

```json
{
  "bin": {
    "nextcloud-cli": "run.js",
    "nxc": "run.js"
  }
}
```

If `run.js` needs to know which alias was invoked, it may inspect `process.argv[1]`
or use separate wrappers. The product behavior must be identical either way.

The npm package should also copy the reference CLI's operational lessons:

- require Node.js 18 or newer so native `fetch` is available
- keep npm dependencies at zero unless proxy support or extraction support forces
  an explicit dependency
- set `preferUnplugged` when needed so package managers do not hide the installed
  binary inside an archive
- write an internal `.version` file after successful binary installation
- sanitize installer failures before printing them
- test unsupported platform messages and missing binary recovery

### 37.7 Cargo packages

The Rust crates should be publishable in this order:

1. `nextcloud`
2. `nextcloud-cli`

Expected install command:

```bash
cargo install nextcloud-cli --locked
```

Cargo package metadata must include description, license, repository, homepage,
readme, keywords, and categories.

### 37.8 Homebrew and other package managers

Homebrew is not required for the first internal MVP, but the release design should
not block it. A future Homebrew formula should download GitHub Release artifacts
and verify checksums.

Future package-manager targets:

- Homebrew
- Nix flake
- AUR
- Docker image for CI and agent runners

These are post-MVP unless explicitly pulled into a release milestone.

### 37.9 Install UX requirements

`nextcloud-cli --version` must work immediately after installation.

`nextcloud-cli --help` must show:

- one-line product summary
- auth quickstart
- common file/share/calendar examples
- link to documentation

First useful flow:

```bash
nextcloud-cli auth login --server https://cloud.example.com
nextcloud-cli files list /
```

Headless useful flow:

```bash
nextcloud-cli auth add \
  --server https://cloud.example.com \
  --user nicholai \
  --app-password "$NEXTCLOUD_APP_PASSWORD"
nextcloud-cli auth status
```


### 37.10 Self-update command

Self-update is required from the start. Users forget to update CLIs, and adding
self-update later is harder because old installations do not know how to update
themselves.

Public commands:

```bash
nextcloud-cli update check
nextcloud-cli update apply [--yes]
```

`nextcloud-cli update check`:

- checks GitHub Releases for the latest compatible version
- prints current version, latest version, install method if known, and update
  availability
- never mutates local files

Expected output:

```json
{
  "current_version": "0.1.0",
  "latest_version": "0.1.1",
  "update_available": true,
  "install_method": "github-release",
  "can_self_update": true
}
```

`nextcloud-cli update apply`:

- downloads the matching release archive for the current platform
- downloads and verifies checksum
- replaces the current binary atomically where possible
- preserves config, credentials, cache, and logs
- refuses to run without `--yes` in non-interactive contexts
- prints a clear fallback when the install method is package-manager owned

Install-method behavior:

| Install method | Self-update behavior |
| --- | --- |
| GitHub Release direct binary | Fully self-update. |
| npm wrapper | Prefer `npm update -g nextcloud-cli`; `update check` may report available version. |
| Cargo install | Print `cargo install nextcloud-cli --locked --force`. |
| Homebrew | Print `brew upgrade nextcloud-cli`. |
| Unknown | Refuse apply and print manual instructions. |

The update implementation should learn from the npm installer in `references/cli`:
platform detection, GitHub Release artifact naming, checksum verification, archive
extraction, sanitized errors, and version tracking.

### 37.11 Self-update completion gate

Self-update is complete only when:

- `update check` works without mutating state
- `update apply --yes` works for direct GitHub Release installs
- package-manager installs receive correct fallback instructions
- checksum mismatch aborts the update
- binary replacement is atomic where the OS allows it
- failed update leaves the old binary usable
- tests cover platform detection, no update available, update available,
  checksum mismatch, package-manager fallback, and sanitized errors


### 37.12 Distribution completion gate

Distribution is complete only when:

- GitHub Release artifacts are produced for every supported target.
- Every artifact has a `.sha256` file.
- npm installer downloads and verifies the correct artifact.
- npm wrapper exposes both `nextcloud-cli` and `nxc`.
- Cargo install works from a published crate or local package dry-run.
- Release notes include direct install instructions.
- Install docs include npm, curl installer, Cargo, and direct release methods.
- A clean machine or container can install and run `nextcloud-cli --version`.
- Installer tests cover platform detection, unsupported platform errors, checksum
  mismatch, missing binary reinstall, and sanitized failure output.

## 38. Installation and Setup Agent Skill Spec

The repository should include an optional agent skill that teaches agents how to
install and configure `nextcloud-cli` on behalf of their users. This skill is part
of the product experience because the issue request is explicitly about AI agent
file access.

Skill path:

```text
skills/nextcloud-cli-setup/SKILL.md
```

### 38.1 Skill purpose

The setup skill helps an agent:

1. detect whether `nextcloud-cli` or `nxc` is already installed
2. choose the safest available install method
3. install or update the CLI
4. verify the binary
5. check whether the installed CLI can self-update
6. configure auth with user consent
7. keep the selected profile in safe agent mode by default
8. run a harmless smoke check
9. explain what was installed and where credentials live

### 38.2 Skill trigger language

The skill should trigger when the user asks things like:

- install Nextcloud CLI
- set up nextcloud-cli
- configure Nextcloud for my agent
- connect to my Nextcloud
- let this agent access my Nextcloud files
- use Nextcloud from the terminal

### 38.3 Skill safety rules

The skill must instruct agents to:

- ask before installing software
- ask before opening a browser-based login flow
- never ask the user to paste their main account password
- prefer Login Flow v2 or app passwords
- never print app passwords back to the user
- never store credentials outside the CLI credential store unless the user
  explicitly requests environment-variable configuration
- run read-only validation after setup
- avoid public share creation during setup

### 38.4 Skill install method preference

The skill should prefer install methods in this order:

1. Existing local binary if version is acceptable.
2. npm install if Node.js 18 or newer is available.
3. Direct GitHub Release download if platform can be detected.
4. Cargo install if Rust is available.
5. Ask the user for manual install if none of the above is available.

This order favors agent environments where npm is common, keeps a simple curl
path for servers and humans, and avoids compiling Rust unless necessary.

### 38.5 Skill setup workflow

The skill should define this workflow:

```text
1. Check `nextcloud-cli --version` and `nxc --version`.
2. If missing, inspect OS, architecture, Node, npm, Cargo, and shell PATH.
3. Pick install method using the preference order.
4. Install the CLI.
5. Verify `nextcloud-cli --version`.
6. Run `nextcloud-cli update check` if network access is appropriate.
7. Ask user for the Nextcloud server URL if unknown.
8. Run `nextcloud-cli auth login --server <url>` for interactive setup, or
   `nextcloud-cli auth add` only when the user provides an app password.
9. Run `nextcloud-cli profiles policy show <profile>` and keep safe defaults.
10. Run `nextcloud-cli auth status`.
11. Run `nextcloud-cli files list /` as a harmless read-only check.
12. Summarize installation path, profile name, policy mode, and next commands.
```

### 38.6 Skill validation commands

The skill should use these validation commands:

```bash
nextcloud-cli --version
nextcloud-cli update check --format json
nextcloud-cli auth status --format json
nextcloud-cli profiles policy show <profile> --format json
nextcloud-cli files list / --format json
```

It may use `nxc` only after confirming the alias exists.

### 38.7 Skill failure handling

The skill must document failure branches for:

- unsupported OS or CPU architecture
- npm missing
- npm install failure
- checksum mismatch
- binary not on PATH
- login flow timeout
- invalid server URL
- revoked app password
- self-signed TLS certificate
- optional apps missing
- self-update unavailable because the install method is package-manager owned
- profile policy denying an attempted setup validation command

The skill should prefer clear next steps over clever recovery. No need to
laminate the pancake.

### 38.8 Skill completion gate

The setup skill is complete only when:

- `skills/nextcloud-cli-setup/SKILL.md` exists.
- basic feature skills exist for files, shares, calendar, contacts, notes, deck,
  and activity.
- It includes install method preference order.
- It includes safety rules.
- It includes validation commands.
- It keeps profile policy in safe agent mode unless the user asks otherwise.
- It includes failure handling.
- It has been tested by following it on at least one clean environment or
  container.
- It does not instruct agents to collect or expose primary account passwords.

## 39. Documentation Requirements

MVP documentation must include:

- README quickstart
- authentication setup
- command reference
- config and migration reference
- proxy, custom CA, and TLS troubleshooting
- JSON examples
- error examples
- optional app behavior
- smoke test instructions
- AI agent usage notes
- security notes
- installation and distribution methods
- setup agent skill usage
- basic feature skill usage for files, shares, calendar, contacts, notes, deck,
  and activity
- share revocation and cleanup examples
- write operation examples for calendar, contacts, notes, and Deck
- terminal art usage and `--no-art` behavior
- profile policy and agent mode guidance
- content search and snippet behavior
- compatibility matrix
- self-update commands and install-method behavior

### 39.1 README quickstart target

```bash
nextcloud-cli auth login --server https://cloud.example.com
nextcloud-cli files list /
nextcloud-cli files search "invoice"
nextcloud-cli calendar events --range 7d
nextcloud-cli activity recent --limit 20
```

### 39.2 Agent usage note

The docs should explicitly say:

- Use JSON mode.
- Prefer read-only commands unless the user asks for changes.
- Do not create public shares without user intent.
- Do not create, update, or delete calendar events, contacts, notes, or Deck cards
  without user intent.
- Treat optional app unavailable errors as normal environment state.
- Use `--dry-run` before write operations when uncertain.
- Check `profiles list` or `auth status` before working across organizations.
- Keep agent-managed profiles in safe mode unless the user explicitly changes
  policy.

## 40. Release Readiness Checklist

A release candidate is ready only when:

- [ ] `docs/SPEC.md` is up to date.
- [ ] README quickstart matches behavior.
- [ ] All public commands have examples.
- [ ] All command examples are tested or manually smoke-verified.
- [ ] `cargo fmt --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `cargo test --workspace` passes.
- [ ] Real-server smoke report exists.
- [ ] Capability snapshot exists in the smoke report.
- [ ] Large transfer path has been tested or explicitly deferred with rationale.
- [ ] Command metadata snapshot is current.
- [ ] Config migration fixtures pass.
- [ ] Proxy/custom CA/TLS behavior is documented and tested.
- [ ] Fixture/snapshot secret scan passes.
- [ ] Secrets are redacted in snapshot tests.
- [ ] Binary name `nextcloud-cli` works.
- [ ] Alias `nxc` works.
- [ ] Version output works.
- [ ] Help output works.
- [ ] Human-facing help or first-run output includes terminal art.
- [ ] JSON and agent-mode outputs contain no terminal art.
- [ ] `--no-art` and `NEXTCLOUD_CLI_NO_ART=1` work.
- [ ] README states the project is unofficial and client-side.
- [ ] Install/package path is documented.
- [ ] GitHub Release artifacts and checksums are produced.
- [ ] npm installer downloads, verifies, and runs the release binary.
- [ ] curl installer downloads, verifies, and installs both binaries.
- [ ] `nextcloud-cli update check` and direct-binary `update apply --yes` are tested.
- [ ] Cargo package dry-run or publish succeeds.
- [ ] `skills/nextcloud-cli-setup/SKILL.md` exists.
- [ ] Basic feature skills exist for files, shares, calendar, contacts, notes,
      deck, and activity.
- [ ] `docs/COMPATIBILITY.md` records tested Nextcloud server versions.
- [ ] `profiles policy show/set/reset` are tested.
- [ ] Content-search behavior is tested or explicitly marked unavailable with a
      tracked blocker.
- [ ] Share cleanup/revoke path is tested in smoke runs.
- [ ] Write operations for calendar, contacts, notes, and Deck are mock-tested.
- [ ] Optional write smoke results are recorded when enabled or skipped.

## 41. Implementation Phasing

The MVP should be built in vertical slices so every milestone leaves behind a
working CLI, not just a wider pile of half-finished modules. Each phase should
include command wiring, JSON output, errors, tests, and documentation updates.

### 41.1 Phase 0: repository spine

Status: complete as of commit `8d3139e`.

Delivered:

- Rust workspace with library and CLI crates
- `nextcloud-cli` binary and `nxc` alias behavior in development
- shared error type and JSON error envelope
- output formatter with JSON default
- config directory resolution
- command metadata skeleton
- fixture directory and test harness
- CI commands for fmt, clippy, and tests

Completion signal:

```bash
nextcloud-cli --version
nextcloud-cli --help --no-art
nextcloud-cli commands schema --format json
```

### 41.2 Phase 1: auth, profiles, and server detection

Status: partial. Login Flow v2, manual app-password auth, profiles,
`auth status`, `server status`, `server capabilities`, `NEXTCLOUD_CLI_PROFILE`,
and capability caching are implemented. OS keyring storage and real-server smoke
validation remain pending.

Deliverables:

- Login Flow v2 auth: implemented through `auth login`; real-server smoke still
  pending
- manual app-password auth: implemented through `auth add`
- secure credential storage abstraction: partial, abstraction, OS keyring backend, and local file fallback
  implemented; target-platform keyring validation pending
- profile selection and ambiguity handling: partial, `--profile`,
  `NEXTCLOUD_CLI_PROFILE`, and stored default profile implemented; multiple-profile
  ambiguity listing remains pending
- `auth status`: implemented
- `server status`: implemented
- `server capabilities`: implemented with `--refresh`
- capability cache shape: implemented per profile with 1 hour TTL

Completion signal:

```bash
nextcloud-cli auth login --server https://cloud.example.com
nextcloud-cli auth status --format json
nextcloud-cli profiles list --format json
nextcloud-cli server capabilities --format json
```

### 41.3 Phase 2: WebDAV files core

Status: partial. The initial WebDAV client, multistatus parser, `files list`,
`files stat`, and `files mkdir --dry-run` are implemented. Upload, download, DAV
search, broader path-encoding coverage, and mock HTTP tests remain pending.

Deliverables:

- remote path normalization and segment encoding: partial
- WebDAV client: partial, PROPFIND and MKCOL implemented
- multistatus parser: implemented with unit coverage
- `files list`: implemented
- `files stat`: implemented
- `files mkdir`: partial, `--dry-run` and MKCOL path implemented
- `files upload` simple PUT path: pending
- `files download` streaming path: pending
- DAV name search: pending

Completion signal:

```bash
nextcloud-cli files mkdir /nextcloud-cli-smoke --dry-run
nextcloud-cli files list / --format json
nextcloud-cli files search report --search-mode name --format json
```

### 41.4 Phase 3: sharing and safety policy

Deliverables:

- OCS client and envelope parser
- `shares list`
- `shares create --public`
- `shares delete` / `shares revoke`
- destructive action confirmation model
- first version of per-profile policy enforcement
- audit event shape for writes

Completion signal:

```bash
nextcloud-cli shares list --format json
nextcloud-cli shares create /path/to/file --public --dry-run --format json
nextcloud-cli profiles policy show personal --format json
```

### 41.5 Phase 4: calendar and contacts

Deliverables:

- CalDAV discovery and range query
- iCalendar parsing and serialization
- calendar create/update/delete with dry-run and ETag behavior
- CardDAV discovery and search
- vCard parsing and serialization
- contacts create/update/delete with dry-run and ETag behavior

Completion signal:

```bash
nextcloud-cli calendar events --range 7d --format json
nextcloud-cli contacts search Ada --format json
nextcloud-cli calendar create --calendar personal --summary Test --starts-at 2026-04-10T16:00:00Z --ends-at 2026-04-10T17:00:00Z --dry-run
```

### 41.6 Phase 5: optional apps

Deliverables:

- Notes read/write commands
- Deck board, stack, and card commands
- Activity feed command
- optional app unavailable mapping
- smoke report fields for enabled and unavailable apps

Completion signal:

```bash
nextcloud-cli notes list --exclude-content --format json
nextcloud-cli deck boards --format json
nextcloud-cli activity recent --limit 20 --format json
```

### 41.7 Phase 6: distribution and agent experience

Status: partial. README, install/config/network/smoke docs, CI, `nxc` alias, and
placeholder `update check` are implemented. Release packaging, npm wrapper, curl
installer, real self-update, skills, and terminal art remain pending.

Deliverables:

- README quickstart: implemented for source/development workflow
- install docs: partial, source install documented
- npm wrapper: pending
- curl installer: pending
- GitHub Release artifact naming and checksum verification: pending
- `update check`: partial, development placeholder implemented
- direct-binary `update apply --yes`: pending
- setup agent skill: pending
- basic feature skills: pending
- terminal art assets and output gating: pending

Completion signal:

```bash
npm install -g nextcloud-cli
nextcloud-cli update check --format json
nextcloud-cli --help
```

### 41.8 Phase discipline

Each phase must leave the repository in a shippable state:

- examples in the changed docs match implemented commands
- command metadata includes new public commands
- mock tests cover success and failure
- secrets are redacted in snapshots
- stdout remains parseable JSON in JSON mode
- `docs/SPEC.md` is updated when behavior changes

## 42. Open Questions for Future Versions

These are intentionally outside MVP decision-making:

1. Should the CLI eventually support OIDC bearer authentication?
2. Should the CLI eventually include sync primitives?
3. How far beyond basic writes should calendar, contacts, notes, and Deck grow?
4. Should raw request commands grow OpenAPI discovery from Nextcloud app
   `openapi.json` files?
5. Should the CLI become a formal Nextcloud app or remain a standalone package?
6. Should agent skills be generated from command metadata or handwritten?

These questions must not block the issue-complete MVP.
