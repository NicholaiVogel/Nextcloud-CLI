# Network Behavior

The initial client uses `reqwest` with rustls TLS, a 10 second connect timeout, a 60 second request timeout, connection pooling, and a `nextcloud-cli/<version>` user agent.

Retries, proxy configuration, custom CA support, and stricter TLS policy controls are tracked in [`SPEC.md`](SPEC.md).
