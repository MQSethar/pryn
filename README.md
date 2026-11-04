# PRYN

**Evidence-based Linux system diagnostics.**

PRYN inspects Linux system interfaces and reports what it can establish about
Wi-Fi, Bluetooth, and audio. It is designed to explain observations, preserve
uncertainty, and recommend safe next steps instead of applying automatic fixes.

## Current status

PRYN is under active development. The current initial implementation provides:

- Linux distribution, kernel, and architecture identification.
- Wi-Fi interface and rfkill discovery through sysfs.
- Bluetooth controller and rfkill discovery through sysfs.
- ALSA sound-card inventory through procfs.
- Human-readable and JSON diagnostic reports.
- Deterministic diagnostic result models and fixture-based tests.

NetworkManager, iwd, BlueZ D-Bus, PipeWire, and PulseAudio provider integrations
are not implemented yet. Their checks are explicitly reported as unknown rather
than being guessed.

## Requirements

- Linux (initially tested on x86_64 distributions).
- The Rust toolchain pinned by rust-toolchain.toml for development.
- No root privileges are required for normal read-only checks.

## Usage

Diagnose all supported subsystems:

```bash
pryn diagnose
```

Diagnose one subsystem:

```bash
pryn diagnose wifi
pryn diagnose bluetooth
pryn diagnose audio
```

Emit machine-readable JSON to standard output:

```bash
pryn diagnose --format json
pryn diagnose wifi --format json > wifi-report.json
```

PRYN does not modify system configuration or enable/disable radios.

## Development

```bash
cargo check --locked
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

See docs/architecture.md and docs/diagnostic-semantics.md.
