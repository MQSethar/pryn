# Contributing to PRYN

PRYN is a Linux diagnostic tool. Changes should preserve the separation between Linux data collection, diagnostic rules, and report presentation.

## Before opening a pull request

```bash
cargo fmt --all
cargo check --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

Or run:

```bash
./scripts/verify.sh
```

## Engineering rules

- Never add `unsafe` without a reviewed design proposal. The crate currently forbids it.
- Do not use `unwrap()` or `expect()` for ordinary fallible operations in production code.
- Do not invoke a shell to read system state. Prefer documented kernel interfaces and carefully controlled process adapters when necessary.
- Keep diagnostic rules free of filesystem, D-Bus, process, and network I/O.
- Missing or inaccessible information must become `unknown`, not `pass`.
- Do not infer internet connectivity from interface state or carrier.
- Keep diagnostics read-only. Any future repair operation requires a separate safety design and explicit user confirmation.
- Add tests for new diagnostic rules using synthetic observations; hardware must not be required for unit tests.
- Keep `Cargo.toml` and `Cargo.lock` changes together in the same pull request.
- Update diagnostic documentation when a finding's meaning or report schema changes.
- Update `docs/installation.md`, `docs/distribution.md`, `docs/support-matrix.md`, and `site/release.json` when package assets or supported distributions change.

## Licensing

PRYN is offered under either the MIT License or Apache License 2.0, at the recipient's option. Before submitting code, make sure you have the right to contribute it under these terms. Contributions are provided under the same dual-license terms unless the maintainers and contributor agree otherwise in writing.

## Pull requests

Use a focused branch and descriptive commits. Explain observed behavior, evidence sources, failure modes, tests added, and any distribution-specific assumptions. Do not merge a failing CI check.

Release and packaging instructions are in [`docs/releasing.md`](docs/releasing.md).
