# Compatibility Matrix

Real-server smoke has started against `https://nextcloud.biohazardvfx.com`.
The Login Flow v2 start endpoint returned a browser approval URL, confirming that
the flow is available. The fully headless app-password path also succeeded over
SSH by reading the account password from an injected environment variable and
storing only the generated app password.

| Server version | Profile | Files | Shares | Calendar | Contacts | Notes | Deck | Activity | Tested at |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29.0.1 | biohazard | list, dry-run mkdir | pending | pending | pending | pending | pending | pending | 2026-04-10 |

Notes:

- `status.php` succeeded and reported Nextcloud `29.0.1.1` / `29.0.1`.
- Login Flow v2 start succeeded and produced an approval URL.
- `auth app-password --password-env` succeeded for the `biohazard` profile.
- `auth status`, `server status`, `server capabilities --refresh`, `files list /`,
  and `files mkdir /nextcloud-cli-smoke --dry-run` succeeded.
- Capability smoke reported 20 top-level capability groups.
