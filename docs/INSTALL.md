# Installation

## Install from source

```bash
cargo install --path crates/nextcloud-cli --locked
```

## Install from a GitHub release

Tagged releases publish native archives for Linux, macOS, and Windows. Pick the
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

Windows release assets are zip files with a matching `.sha256` checksum file.

## Planned installers

The npm wrapper, curl installer, Homebrew tap, and Cargo publishing support are
specified in [`SPEC.md`](SPEC.md) and will land after the binary release path is
proven.
