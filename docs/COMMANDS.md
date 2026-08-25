# Command Surface

Generated command metadata is available from the CLI:

```bash
nextcloud-cli commands schema --format json
```

The current implementation covers auth/profile/server detection, core WebDAV
file commands, profile-scoped visual media indexing and search, shares, calendar, contacts, Activity, Notes, Deck, smoke checks,
and update checks:

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
- `profiles policy show <name>`
- `profiles policy set <name> [--agent-mode true|false] [--default-dry-run true|false] [--allow-destructive true|false] [--allow-public-shares true|false]`
- `profiles policy reset <name> --yes`
- `server status`
- `server capabilities [--refresh]`
- `files list [path]`
- `files search <query> [--path <scope>] [--limit <n>] [--search-mode name]`
- `files search-image <local-image> [--path <scope>] [--media images|videos|all] [--limit <n>] [--video-candidates <n>]`
- `index status`
- `index build [--path <remote-path>] [--media images|videos|all] [--max-files <n>] [--video-sample-rate <n>]`
- `index update [--path <remote-path>] [--media images|videos|all] [--max-files <n>] [--video-sample-rate <n>]`
- `index clear --yes`
- `files stat <path>`
- `files mkdir <path> [--parents] [--dry-run]`
- `files upload <local> <remote> [--overwrite] [--content-type <mime>]`
- `files download <remote> <local> [--overwrite]`
- `files delete <path> [--dry-run] --yes`
- `shares list [--path <path>] [--shared-with-me] [--include-tags]`
- `shares create <path> --public [--password <password>] [--expire-date <yyyy-mm-dd>] [--dry-run] [--yes]`
- `shares delete <share-id> [--dry-run] [--yes]`
- `shares revoke <share-id> [--dry-run] [--yes]`
- `calendar events [--date today|<yyyy-mm-dd>] [--range <days>d] [--from <date-or-rfc3339>] [--to <date-or-rfc3339>] [--calendar <name>]`
- `calendar create --calendar <name> --summary <text> --starts-at <date-or-rfc3339> --ends-at <date-or-rfc3339> [--location <text>] [--description <text>] [--all-day] [--dry-run]`
- `calendar delete --calendar <name> <uid> [--dry-run] [--yes]`
- `contacts search <query> [--limit <n>] [--addressbook <name>]`
- `contacts create --addressbook <name> --full-name <name> [--email <email>] [--phone <phone>] [--organization <text>] [--dry-run]`
- `contacts delete --addressbook <name> <uid> [--dry-run] [--yes]`
- `activity recent [--limit <n>]`
- `notes list [--category <name>] [--exclude-content] [--limit <n>]`
- `notes create --title <title> [--content <markdown>] [--from-file <path>] [--category <name>] [--dry-run]`
- `notes update <note-id> [--title <title>] [--content <markdown>] [--from-file <path>] [--dry-run]`
- `notes delete <note-id> [--dry-run] [--yes]`
- `deck boards [--details]`
- `deck boards create --title <title> [--color <hex>] [--dry-run]`
- `deck stacks create --board <id> --title <title> [--order <n>] [--dry-run]`
- `deck cards --board <id> [--include-archived]`
- `deck cards create --board <id> --stack <id> --title <title> [--description <markdown>] [--due-at <datetime>] [--order <n>] [--dry-run]`
- `deck cards update <card-id> --board <id> --stack <id> [--title <title>] [--description <markdown>] [--due-at <datetime>] [--order <n>] [--dry-run]`
- `deck cards move <card-id> --board <id> --from-stack <id> --to-stack <id> [--order <n>] [--dry-run]`
- `deck cards archive <card-id> --board <id> --stack <id> [--dry-run] [--yes]`
- `deck cards delete <card-id> --board <id> --stack <id> [--dry-run] [--yes]`
- `smoke run [--files-path <path>] [--skip-optional] [--calendar-days <n>] [--contacts-query <query>]`
- `update check`
