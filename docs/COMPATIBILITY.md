# Compatibility Matrix

Initial real-server smoke has started against `https://nextcloud.biohazardvfx.com`.
The Login Flow v2 start endpoint returned a browser approval URL, confirming that
the flow is available. The approval window timed out before credentials were
issued, so authenticated command smoke is still pending.

The CLI now also has a fully headless `auth app-password --password-stdin` path
for SSH-only sessions where no browser approval is possible on the working
machine. Real-server validation for that path is pending operator-entered
account-password input.

| Server version | Profile | Files | Shares | Calendar | Contacts | Notes | Deck | Activity | Tested at |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 29.0.1 | unauth-smoke | pending | pending | pending | pending | pending | pending | pending | 2026-04-10 |

Notes:

- `status.php` succeeded and reported Nextcloud `29.0.1.1` / `29.0.1`.
- Login Flow v2 start succeeded and produced an approval URL.
- Authenticated capability and files smoke remain pending headless app-password
  setup or browser approval from another machine.
