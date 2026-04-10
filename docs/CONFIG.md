# Configuration

`nextcloud-cli` stores profile metadata under the platform config directory by default:

```text
$XDG_CONFIG_HOME/nextcloud-cli/config.json
```

Override it with either:

```bash
nextcloud-cli --config-dir /path/to/config config path
NEXTCLOUD_CLI_CONFIG_DIR=/path/to/config nextcloud-cli config path
```

`config.json` contains profile metadata only. App passwords are stored through the credential backend and are intentionally excluded from profile output.


## Profile selection

Profile selection currently follows this order for implemented commands:

1. `--profile <name>`
2. `NEXTCLOUD_CLI_PROFILE`
3. stored default profile

## Credentials

The credential store is abstracted behind backend selection. By default, the CLI
uses `keyring-auto`: it tries the operating-system keyring first and falls back to
the local file backend when no usable keyring is available, which is common in
headless development containers.

Force a backend with:

```bash
NEXTCLOUD_CLI_KEYRING_BACKEND=keyring nextcloud-cli auth status
NEXTCLOUD_CLI_KEYRING_BACKEND=file nextcloud-cli auth status
```

The local file backend stores credentials here:

```text
$XDG_CONFIG_HOME/nextcloud-cli/credentials.json
```

On Unix this file is written with owner-only permissions. App passwords are not
written to `config.json` or normal command output.
