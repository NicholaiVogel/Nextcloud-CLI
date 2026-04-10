# Smoke Tests

Local spine checks:

```bash
cargo run -p nextcloud-cli -- --version
cargo run -p nextcloud-cli -- --help --no-art
cargo run -p nextcloud-cli -- commands schema --format json
cargo test --workspace
```

Against a real server:

```bash
NEXTCLOUD_APP_PASSWORD=... cargo run -p nextcloud-cli -- auth add \
  --server https://cloud.example.com \
  --user you \
  --profile personal

cargo run -p nextcloud-cli -- --profile personal server status
cargo run -p nextcloud-cli -- --profile personal server capabilities
cargo run -p nextcloud-cli -- --profile personal files list /
cargo run -p nextcloud-cli -- --profile personal files mkdir /nextcloud-cli-smoke --dry-run
```
