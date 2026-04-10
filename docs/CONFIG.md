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

The current credential store is abstracted in code and uses a local file backend:

```text
$XDG_CONFIG_HOME/nextcloud-cli/credentials.json
```

On Unix this file is written with owner-only permissions. App passwords are not
written to `config.json` or normal command output. OS keyring support remains
pending.
