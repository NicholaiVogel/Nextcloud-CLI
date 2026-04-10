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
