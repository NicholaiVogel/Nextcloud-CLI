# Network Behavior

The client uses `reqwest` with rustls TLS, a 10 second connect timeout, a 60
second request timeout, connection pooling, and a `nextcloud-cli/<version>` user
agent. System trust roots and hostname verification are enabled by default.

## Private certificate authorities

Add a PEM-encoded CA bundle for one invocation with:

```bash
nxc --ca-bundle /etc/ssl/nextcloud-ca.pem --profile personal server status
```

The same setting can be provided through `NEXTCLOUD_CLI_CA_BUNDLE`. The bundle
is added to (not substituted for) the system trust roots. It only establishes
trust; the server name must still appear in the certificate SAN. A self-signed
certificate issued for `localhost` will therefore not become valid when the
server is reached through a Tailscale IP.

Unreadable or malformed bundles fail before a request is sent with a structured
`tls_ca_bundle_read_failed` or `tls_ca_bundle_invalid` error.

## TLS diagnostics and emergency bypass

Certificate failures use stable error codes:

- `tls_certificate_untrusted`: use `--ca-bundle` or
  `NEXTCLOUD_CLI_CA_BUNDLE` for the issuing CA.
- `tls_hostname_mismatch`: use a server URL covered by the certificate SAN.
- `tls_handshake_failed`: inspect the server's TLS configuration.

`--insecure` is an explicit, command-local emergency override. It disables
certificate and hostname verification, prints a warning on stderr, is never
stored in a profile, and is rejected for profiles with agent mode enabled. It
should not be used as the normal fix.

Proxy environment variables are handled by reqwest's default proxy behavior;
proxy credentials are never included in diagnostics.
