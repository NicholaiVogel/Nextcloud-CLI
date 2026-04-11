# nextcloud-cli npm wrapper

This package installs the native `nextcloud-cli` Rust binary and exposes both
command names:

```bash
nxc --help
nextcloud-cli --help
```

The project is unofficial and talks to existing Nextcloud client APIs.

During install, the package downloads the matching GitHub release archive,
verifies its SHA-256 checksum, and stores the native binaries under
`vendor/<platform>/`.

For local development, skip the download and point the wrapper at a checkout
binary:

```bash
NEXTCLOUD_CLI_SKIP_DOWNLOAD=1 npm install
NEXTCLOUD_CLI_BINARY=/path/to/target/debug/nextcloud-cli nxc --help
```
