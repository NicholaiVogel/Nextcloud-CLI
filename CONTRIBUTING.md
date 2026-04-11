# Contributing

Thanks for helping improve `nxc`.

This project is an unofficial client-side CLI for Nextcloud, written in Rust.
It is early, but the shape is intentional: predictable JSON, safe credential
handling, and commands that work for humans, shell scripts, and AI agents.

## Before you start

Read the spec first:

- [`docs/SPEC.md`](docs/SPEC.md), product contract and implementation phases
- [`docs/COMMANDS.md`](docs/COMMANDS.md), implemented command surface
- [`docs/CONFIG.md`](docs/CONFIG.md), config and credential behavior
- [`docs/SMOKE.md`](docs/SMOKE.md), manual smoke testing

If a change affects user-visible behavior, update the relevant documentation in
the same pull request.

## Development setup

```bash
git clone https://github.com/NicholaiVogel/Nextcloud-CLI.git
cd Nextcloud-CLI
cargo build --workspace
```

Run the CLI from the checkout:

```bash
cargo run -p nextcloud-cli --bin nxc -- commands schema
cargo run -p nextcloud-cli --bin nxc -- config doctor
```

Install locally:

```bash
cargo install --path crates/nextcloud-cli --locked
```

## Validation

Run these before opening a pull request:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If your change touches live Nextcloud behavior, run the relevant smoke tests in
[`docs/SMOKE.md`](docs/SMOKE.md). Do not commit real server URLs, usernames,
passwords, app passwords, tokens, or private file contents.

## Coding guidelines

- Keep stdout machine-readable for JSON mode.
- Send diagnostics and progress to stderr.
- Never print secrets.
- Do not store app passwords in `config.json`.
- Use structured error codes.
- Make destructive operations require `--dry-run` or explicit confirmation.
- Preserve profile and server information in write outputs.
- Add tests for protocol parsing, request construction, and safety behavior.

Exported Rust functions should have explicit return types.

## Documentation guidelines

- Keep README content accurate to the implemented state.
- Put planned behavior in `docs/SPEC.md`, not in quick-start instructions.
- Prefer concrete examples over broad claims.
- Document safety behavior alongside commands that write, delete, or disclose
  data.

## Pull requests

Good pull requests are small, focused, and easy to validate.

Include:

- what changed
- why it changed
- tests run
- any real-server smoke coverage, with secrets redacted
- documentation updates when behavior changes

## Security

Please read [`SECURITY.md`](SECURITY.md) before reporting vulnerabilities. Do
not post credentials, private URLs, or exploit details in public issues.
