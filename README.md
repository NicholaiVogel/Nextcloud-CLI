# nextcloud-cli

Unofficial client-side CLI for Nextcloud, built for humans, scripts, and AI agents.

This repository is in the first implementation pass. The canonical product contract lives in [`docs/SPEC.md`](docs/SPEC.md).

## Current spine

```bash
cargo run -p nextcloud-cli -- commands schema
cargo run -p nextcloud-cli -- config doctor
cargo run -p nextcloud-cli -- auth add \
  --server https://cloud.example.com \
  --user you \
  --app-password "$NEXTCLOUD_APP_PASSWORD"
cargo run -p nextcloud-cli -- auth status
```

The CLI defaults to JSON output. Secrets are not stored in `config.json`; the initial credential backend writes a `0600` local credential file so command contracts can be exercised while the secure keyring backend is added.

## Development

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
