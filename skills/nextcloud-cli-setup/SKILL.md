---
name: nextcloud-cli-setup
description: Install, verify, and configure Nextcloud CLI for a human, script, or AI agent. Use when setting up nxc or nextcloud-cli, connecting an agent to a Nextcloud server, configuring profiles and credentials, validating auth, or running safe first smoke checks in local, SSH, CI, or headless environments.
---

# Nextcloud CLI setup

Use this skill when the task is to install `nextcloud-cli`, configure `nxc`,
connect to a Nextcloud server, or prepare a safe profile for an agent.

Prefer the short command name `nxc` once it is confirmed available. Use
`nextcloud-cli` when clarity matters or when testing both aliases.

## Core safety rules

- Ask before installing software.
- Ask before opening an interactive browser login flow.
- Do not echo account passwords, app passwords, share links, recovery tokens, or
  credential file contents.
- Prefer Login Flow v2 on machines with browser access.
- Prefer an existing app password or headless app-password minting on SSH hosts.
- Keep profile policy in safe agent mode unless the user explicitly asks for a
  broader policy.
- Validate setup with read-only commands first.
- Do not create public shares during setup.
- Avoid `--insecure`. If a self-signed certificate blocks setup, explain the CA
  fix instead of silently weakening TLS.

## Install decision order

1. Use an existing `nxc` or `nextcloud-cli` binary if it works.
2. Use npm when `npm` is available.
3. Use the curl installer when npm is unavailable or the user prefers a simple
   shell install.
4. Use `cargo install --path` from a checkout for development.
5. Give manual release-download instructions if none of the above fit.

## Detect the environment

```bash
command -v nxc || true
command -v nextcloud-cli || true
nxc --version 2>/dev/null || nextcloud-cli --version 2>/dev/null || true
node --version 2>/dev/null || true
npm --version 2>/dev/null || true
cargo --version 2>/dev/null || true
uname -s
uname -m
```

If `nxc` exists and returns a version, use it for the rest of the workflow.

## Install options

npm is the default for most agent and SSH environments:

```bash
npm install -g nextcloud-cli
nxc --version
nextcloud-cli --version
```

The curl installer is useful on servers and minimal shells:

```bash
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | sh
nxc --version
```

Install into a specific directory when needed:

```bash
curl -fsSL https://raw.githubusercontent.com/NicholaiVogel/Nextcloud-CLI/main/install.sh | INSTALL_DIR="$HOME/.local/bin" sh
```

From a source checkout:

```bash
cargo install --path crates/nextcloud-cli --locked
nxc --version
```

## Configure authentication

Interactive desktop flow:

```bash
nxc auth login \
  --server https://cloud.example.com \
  --profile personal \
  --set-default
```

Headless account-password flow. Use this only when the user has already arranged
for a secret manager or shell session to provide the password outside the chat.
Do not ask them to paste their primary account password into the conversation.
The CLI reads from stdin and stores only the generated app password:

```bash
printf '%s' "$NEXTCLOUD_PASSWORD" | nxc auth app-password \
  --server https://cloud.example.com \
  --user "User Name" \
  --profile personal \
  --password-stdin \
  --set-default
```

Existing app password from a secret manager:

```bash
NEXTCLOUD_APP_PASSWORD="$APP_PASSWORD" nxc auth add \
  --server https://cloud.example.com \
  --user "User Name" \
  --profile personal \
  --set-default
```

Unset temporary secrets immediately after use:

```bash
unset NEXTCLOUD_PASSWORD APP_PASSWORD NEXTCLOUD_APP_PASSWORD
```

## Safe validation

Run these after install and auth:

```bash
nxc --version
nxc update check --format json
nxc config doctor --format json
nxc profiles list --format json
nxc profiles policy show personal --format json
nxc --profile personal auth status --format json
nxc --profile personal server status --format json
nxc --profile personal files list / --format json
nxc --profile personal smoke run --skip-optional --format json
```

Summarize results. Do not paste raw output if it contains names, paths, emails,
server URLs, or private file metadata the user did not ask to see.

## Profile policy

Inspect policy before changing it:

```bash
nxc profiles policy show personal --format json
```

Keep defaults for agents. Only loosen policy when the user asks for a specific
capability:

```bash
nxc profiles policy set personal --allow-public-shares true
nxc profiles policy set personal --allow-destructive true
nxc profiles policy reset personal --yes
```

## Failure handling

- Unsupported platform: use source build or direct GitHub release guidance.
- npm missing: use the curl installer.
- npm install failed: collect sanitized npm error text and try curl only if the
  user accepts a fallback.
- Binary not on PATH: check `$HOME/.local/bin`, npm global bin, and shell init.
- Checksum mismatch: stop. Do not run the downloaded binary.
- Login flow timeout: retry or switch to headless app-password setup.
- Invalid server URL: verify scheme and host with `server status`.
- Revoked credential: run auth again with a fresh app password.
- Self-signed TLS: configure a trusted CA or proxy settings. Do not normalize
  `--insecure` as an agent default.
- Optional app missing: continue setup. Optional apps are not required for core
  files access.
- Update unavailable: explain that package-manager installs are updated through
  the package manager.

## Completion checklist

- `nxc --version` works.
- `nextcloud-cli --version` works.
- A profile exists and is selected deliberately.
- `auth status` succeeds for the selected profile.
- `files list /` succeeds as a read-only check.
- Profile policy remains safe unless the user explicitly changed it.
- The user knows the install method, install path if relevant, and next commands.
