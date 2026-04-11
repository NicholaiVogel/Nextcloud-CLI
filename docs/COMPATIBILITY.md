# Compatibility Matrix

Real-server smoke has started against `https://nextcloud.biohazardvfx.com`.
The Login Flow v2 start endpoint returned a browser approval URL, confirming that
the flow is available. The fully headless app-password path also succeeded over
SSH by reading the account password from an injected environment variable and
storing only the generated app password.

| Server version | Profile | Files | Shares | Calendar | Contacts | Notes | Deck | Activity | Tested at |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29.0.1 | biohazard | list, mkdir, upload, stat, download, delete | pending | pending | pending | pending | pending | pending | 2026-04-10 |

Notes:

- `status.php` succeeded and reported Nextcloud `29.0.1.1` / `29.0.1`.
- Login Flow v2 start succeeded and produced an approval URL.
- `auth app-password --password-env` succeeded for the `biohazard` profile.
- `auth status`, `server status`, `server capabilities --refresh`, `files list /`,
  `files mkdir --parents`, `files upload`, `files stat`, `files download`, and
  `files delete --dry-run` succeeded.
- `files delete --yes` removed `/nextcloud-cli-smoke-20260410T233520Z`; a
  follow-up `files stat` returned 404, confirming cleanup.
- Downloaded fixture bytes matched the uploaded fixture.
- Capability smoke reported 20 top-level capability groups.
