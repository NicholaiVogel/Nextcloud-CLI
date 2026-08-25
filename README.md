<h1 align="center">
  <img src="https://nextcloud.com/c/uploads/2025/10/Nextcloud-logo-blue.png?original" alt="Nextcloud" height="96" align="center" />
  &nbsp;nxc
</h1>

<p align="center">
  <strong>Give agents and scripts a safe handle on your Nextcloud.</strong>
</p>

<p align="center">
  An unofficial, client-side CLI for files, shares, calendars, contacts, Notes,
  Deck, Activity, and the workflows people actually want to automate.
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/nextcloud-cli"><img alt="npm" src="https://img.shields.io/npm/v/nextcloud-cli?color=0082c9"></a>
  <a href="https://github.com/NicholaiVogel/Nextcloud-CLI/releases"><img alt="GitHub release" src="https://img.shields.io/github/v/release/NicholaiVogel/Nextcloud-CLI?color=0082c9"></a>
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
</p>

> [!NOTE]
> This is an unofficial community project. It runs client-side against existing
> Nextcloud APIs.

`nxc` makes an existing Nextcloud account usable from the terminal, SSH, CI, and
AI agent runtimes. It uses normal Nextcloud HTTP APIs, stores app passwords
locally, returns stable JSON, and puts guardrails around writes.

Install it:

```bash
npm install -g nextcloud-cli
```

Use either command name:

```bash
nxc --help
nextcloud-cli --help
```

## Why this exists

People keep asking for a way to let tools and agents work with their Nextcloud
files directly. Nextcloud already has the storage, calendars, contacts, notes,
boards, activity, users, permissions, and self-hosted ownership model. The
missing piece is a clean command surface agents can trust.

`nxc` is that command surface.

It gives you:

- **Local control:** credentials stay on the machine running the CLI.
- **Agent-ready output:** JSON by default, command metadata, and redacted smoke
  reports.
- **Useful coverage:** files, local visual media search, shares, calendar, contacts,
  Notes, Deck, Activity, profiles, capabilities, and smoke checks.
- **Guarded writes:** dry-run previews, explicit confirmation, profile policy,
  and secret-redacted audit logs.
- **Headless setup:** browser login for desktops, app-password flows for SSH and
  server environments.
- **Portable invocation:** short `nxc` for daily use, explicit `nextcloud-cli`
  for scripts and documentation.

## What it can do today

| Area | Current support |
| --- | --- |
| Files | List, name-search, visual-search, stat, mkdir, upload, download, guarded delete |
| Shares | List, create public links, delete, revoke, preview sensitive actions |
| Calendar | List events, create events, guarded delete |
| Contacts | Search, create, guarded delete |
| Notes | List, create, update, guarded delete |
| Deck | List boards/cards, create boards/stacks/cards, update, move, archive, delete |
| Activity | Read recent activity |
| Agents | JSON output, command schema, agent skills, redacted smoke reports |
| Safety | OS keyring, local file fallback, dry-run, confirmation, profile policy |

## Quick start

```bash
nxc auth login \
  --server https://cloud.example.com \
  --profile personal

nxc --profile personal server status
nxc --profile personal files list /
```

For SSH and headless machines, use the app-password setup flow documented in
[`docs/USAGE.md`](docs/USAGE.md).

To search by visual similarity, build the explicit local index first:

```bash
nxc --profile personal index build --path /Projects --media all --format json
nxc --profile personal files search-image ./reference-frame.png \
  --path /Projects --media all --format json
```


## Install

### npm

```bash
npm install -g nextcloud-cli
```

### curl

```bash
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | sh
```

### GitHub Releases

Download a native archive from
[`releases`](https://github.com/NicholaiVogel/Nextcloud-CLI/releases), verify
the matching `.sha256` file, and put both binaries on your `PATH`.

More install options live in [`docs/INSTALL.md`](docs/INSTALL.md).

## Built for agents

`nxc` treats agent use as a first-class product requirement.

```bash
nxc commands schema --format json
nxc --profile personal smoke run --skip-optional --format json
```

The repository also ships agent skills for setup, files, shares, calendar,
contacts, Notes, Deck, and Activity:

```text
skills/
```

Those skills teach agents how to install the CLI, choose safe auth flows, avoid
leaking secrets, prefer dry-runs, and summarize private data safely.

## Safety model

`nxc` is designed for boring, inspectable automation.

- Normal config excludes account passwords.
- App passwords are stored through the OS keyring when available.
- SSH and containers fall back to an owner-only local credential file.
- Destructive commands require `--dry-run` or `--yes`.
- Public share creation is gated by profile policy.
- Write audit logs redact secrets.
- Private CAs are supported with `--ca-bundle <path>` or
  `NEXTCLOUD_CLI_CA_BUNDLE`; hostname verification remains enabled.
- `--insecure` is a noisy, non-persistent emergency override and is blocked for
  agent-managed profiles.

This matters because agents should be useful around private data, and they need
a smaller blast radius than a raw API token and a prayer.

## Project status

`nxc` is pre-1.0 and already usable. The first release is published on GitHub and
npm. The command surface may still change while the CLI hardens across more
Nextcloud versions and optional app combinations.

Current focus:

- broader compatibility testing
- Homebrew distribution
- real `update apply`
- additional release targets
- more polished human output

Future feature planning lives in [`docs/ROADMAP.md`](docs/ROADMAP.md).

## Documentation

- [`docs/USAGE.md`](docs/USAGE.md), authentication, profiles, workflows, output,
  environment variables, troubleshooting
- [`docs/INSTALL.md`](docs/INSTALL.md), install methods and release artifacts
- [`docs/COMMANDS.md`](docs/COMMANDS.md), implemented command surface
- [`docs/CONFIG.md`](docs/CONFIG.md), configuration and credential behavior
- [`docs/ROADMAP.md`](docs/ROADMAP.md), future feature planning
- [`docs/SMOKE.md`](docs/SMOKE.md), safe real-server validation
- [`docs/NETWORK.md`](docs/NETWORK.md), network, proxy, and TLS behavior
- [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md), tested Nextcloud servers
- [`docs/SPEC.md`](docs/SPEC.md), full product spec and implementation plan
- [`skills/`](skills/), agent skills for common workflows
- [`CONTRIBUTING.md`](CONTRIBUTING.md), contribution guidelines
- [`SECURITY.md`](SECURITY.md), vulnerability reporting
- [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md), community standards

## License

MIT. See [`LICENSE`](LICENSE).
