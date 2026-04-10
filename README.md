# nextcloud-cli

Unofficial client-side CLI for Nextcloud, built for humans, scripts, and AI agents.

This repository is in the first implementation pass. The canonical product contract lives in [`docs/SPEC.md`](docs/SPEC.md).

## Current spine

```bash
cargo run -p nextcloud-cli -- commands schema
cargo run -p nextcloud-cli -- config doctor
cargo run -p nextcloud-cli -- auth login --server https://cloud.example.com --no-open
cargo run -p nextcloud-cli -- auth app-password \
  --server https://cloud.example.com \
  --user you \
  --password-stdin
cargo run -p nextcloud-cli -- auth add \
  --server https://cloud.example.com \
  --user you \
  --app-password "$NEXTCLOUD_APP_PASSWORD"
cargo run -p nextcloud-cli -- auth status
cargo run -p nextcloud-cli -- server capabilities --refresh
cargo run -p nextcloud-cli -- files list /
cargo run -p nextcloud-cli -- files upload ./summary.md /Documents/summary.md
cargo run -p nextcloud-cli -- files download /Documents/brief.md ./brief.md
```

The CLI defaults to JSON output. Secrets are not stored in `config.json`; the
credential backend uses the operating-system keyring when available and falls
back to an owner-only local credential file in headless environments.

`auth login --no-open` works over SSH if you can approve the printed URL from
another browser. `auth app-password --password-stdin` is the fully headless path:
it uses the account password only long enough to mint an app password through
Nextcloud's OCS endpoint, then stores the app password through the credential
backend.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
