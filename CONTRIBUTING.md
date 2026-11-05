# Contributing to PRYN

PRYN is an early-stage Linux diagnostic tool. Changes should preserve the separation between Linux data collection, diagnostic rules, and report presentation.

## Before opening a pull request

```bash
cargo fmt --all
cargo check --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

Or run `./scripts/verify.sh`.

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

## Pull requests

Use a focused branch and a descriptive commit. Explain the observed behavior, the evidence source, failure modes, tests added, and any distribution-specific assumptions. Do not merge a failing CI check.
