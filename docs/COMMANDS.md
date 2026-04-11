# Command Surface

Generated command metadata is available from the CLI:

```bash
nextcloud-cli commands schema --format json
```

The current implementation covers the repository spine, auth/profile/server
detection, core WebDAV file-transfer commands, and initial OCS share listing:

- `commands schema`
- `config path`
- `config show`
- `config doctor`
- `auth login`
- `auth app-password`
- `auth add`
- `auth status`
- `profiles list`
- `profiles show <name>`
- `profiles set-default <name>`
- `server status`
- `server capabilities [--refresh]`
- `files list [path]`
- `files search <query> [--path <scope>] [--limit <n>] [--search-mode name]`
- `files stat <path>`
- `files mkdir <path> [--parents] [--dry-run]`
- `files upload <local> <remote> [--overwrite] [--content-type <mime>]`
- `files download <remote> <local> [--overwrite]`
- `files delete <path> [--dry-run] --yes`
- `shares list [--path <path>] [--shared-with-me] [--include-tags]`
- `shares create <path> --public [--password <password>] [--expire-date <yyyy-mm-dd>] [--dry-run] [--yes]`
- `update check`
