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
cargo run -p nextcloud-cli -- auth login \
  --server https://cloud.example.com \
  --profile personal \
  --no-open

read -rsp "Nextcloud password: " NC_PASSWORD; echo
printf '%s' "$NC_PASSWORD" | cargo run -p nextcloud-cli -- auth app-password \
  --server https://cloud.example.com \
  --user you \
  --profile personal \
  --password-stdin
unset NC_PASSWORD

NEXTCLOUD_APP_PASSWORD=... cargo run -p nextcloud-cli -- auth add \
  --server https://cloud.example.com \
  --user you \
  --profile personal

cargo run -p nextcloud-cli -- --profile personal server status
cargo run -p nextcloud-cli -- --profile personal server capabilities --refresh
cargo run -p nextcloud-cli -- --profile personal server capabilities
cargo run -p nextcloud-cli -- --profile personal files list /
cargo run -p nextcloud-cli -- --profile personal files mkdir /nextcloud-cli-smoke --dry-run

STAMP=$(date -u +%Y%m%dT%H%M%SZ)
REMOTE_DIR="/nextcloud-cli-smoke-$STAMP"
REMOTE_FILE="$REMOTE_DIR/hello file #1.txt"
LOCAL_SRC=$(mktemp)
LOCAL_DST=$(mktemp -u)
printf 'nextcloud-cli smoke %s\n' "$STAMP" > "$LOCAL_SRC"

cargo run -p nextcloud-cli -- --profile personal files mkdir "$REMOTE_DIR" --parents
cargo run -p nextcloud-cli -- --profile personal files upload "$LOCAL_SRC" "$REMOTE_FILE" --content-type text/plain
cargo run -p nextcloud-cli -- --profile personal files stat "$REMOTE_FILE"
cargo run -p nextcloud-cli -- --profile personal files search "hello file" --path "$REMOTE_DIR" --limit 10
cargo run -p nextcloud-cli -- --profile personal files download "$REMOTE_FILE" "$LOCAL_DST"
cmp "$LOCAL_SRC" "$LOCAL_DST"
cargo run -p nextcloud-cli -- --profile personal files delete "$REMOTE_DIR" --dry-run
cargo run -p nextcloud-cli -- --profile personal files delete "$REMOTE_DIR" --yes
rm -f "$LOCAL_SRC" "$LOCAL_DST"
```
