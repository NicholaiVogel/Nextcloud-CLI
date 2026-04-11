# Installation

## Install from source

```bash
cargo install --path crates/nextcloud-cli --locked
```

## Install from a GitHub release

Tagged releases publish native `.tar.gz` archives for Linux, macOS, and Windows. Pick the
archive matching your host triple from
[`releases`](https://github.com/NicholaiVogel/Nextcloud-CLI/releases), verify
the checksum, then place both binaries on your `PATH`.

```bash
version=0.1.0
host=x86_64-unknown-linux-gnu
base="nextcloud-cli-${version}-${host}"

curl -LO "https://github.com/NicholaiVogel/Nextcloud-CLI/releases/download/v${version}/${base}.tar.gz"
curl -LO "https://github.com/NicholaiVogel/Nextcloud-CLI/releases/download/v${version}/${base}.tar.gz.sha256"
sha256sum -c "${base}.tar.gz.sha256"

tar -xzf "${base}.tar.gz"
sudo install -m 0755 "${base}/nextcloud-cli" /usr/local/bin/nextcloud-cli
sudo install -m 0755 "${base}/nxc" /usr/local/bin/nxc
```

Windows release assets are also `.tar.gz` archives with `.exe` binaries inside.

## npm wrapper

The npm package is published as `nextcloud-cli` and exposes both command names:

```bash
npm install -g nextcloud-cli
nxc --help
nextcloud-cli --help
```

During installation, the package downloads the matching GitHub release archive,
verifies its SHA-256 checksum, and stores the native binaries under its package
directory.

Local development test:

```bash
cd npm/nextcloud-cli
NEXTCLOUD_CLI_SKIP_DOWNLOAD=1 npm install
NEXTCLOUD_CLI_BINARY=../../target/debug/nextcloud-cli npm exec -- nxc --help
```

## Planned installers

The curl installer, Homebrew tap, and Cargo publishing support are specified in
[`SPEC.md`](SPEC.md) and will land after the binary release path is proven.
