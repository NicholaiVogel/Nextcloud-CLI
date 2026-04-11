# Command Surface

Generated command metadata is available from the CLI:

```bash
nextcloud-cli commands schema --format json
```

The current implementation covers the repository spine, the first
auth/profile/server detection slice, and the core WebDAV file-transfer commands:

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
- `files stat <path>`
- `files mkdir <path> [--parents] [--dry-run]`
- `files upload <local> <remote> [--overwrite] [--content-type <mime>]`
- `files download <remote> <local> [--overwrite]`
- `files delete <path> [--dry-run] --yes`
- `update check`
