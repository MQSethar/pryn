# PRYN

**Simple system checks for Linux.**

PRYN checks Wi-Fi, Bluetooth, and audio. It shows what it can confirm, explains what may be wrong, and suggests safe next steps. It only reads system information; it does not change your settings.

> **Development status:** early version. PRYN can find Wi-Fi adapters, Bluetooth controllers, sound cards, visible drivers, and radio block state. Wi-Fi also has read-only NetworkManager integration for manager availability, radio switches, adapter connection state, and selected firmware/plugin signals. When NetworkManager has a configured and enabled connectivity check, PRYN reads its last system-wide result. That result may be stale and may reflect another interface, not Wi-Fi. Direct iwd inspection, IP configuration, Bluetooth service state, and desktop audio routing are still being built.

## What works today

- Finds wireless network interfaces.
- Checks whether a wireless interface is up and whether a link is present.
- Checks whether Wi-Fi or Bluetooth radios are blocked.
- Reads NetworkManager over the system D-Bus without changing settings.
- Reads NetworkManager's last connectivity result only when its check is configured and enabled; the result is system-wide, may be stale, and is not Wi-Fi-specific.
- Checks NetworkManager's global networking state, Wi-Fi software/hardware switches, and Wi-Fi device connection state.
- Identifies NetworkManager-reported missing firmware or device-plugin signals.
- Detects iwd, but does not yet inspect its connection state directly.
- Finds Bluetooth controllers.
- Finds ALSA sound cards.
- Shows results in the terminal or saves them as JSON.
- Runs without root privileges. PRYN itself does not send internet requests; NetworkManager may perform its own configured connectivity check independently.

## What is coming next

- Add direct iwd integration.
- Check IP addresses, routes, DNS, and optional connectivity reachability.
- Check the Bluetooth service and connected devices.
- Check audio services, speakers, microphones, and app audio.
- Add an interactive terminal screen.

## Requirements for building PRYN

- Linux (x86_64 is the initial development target).
- The Rust version listed in `rust-toolchain.toml`.
- Cargo (installed with Rustup).

## Build

```bash
cargo check --locked
cargo build --release --locked
```

Run all local quality checks with:

```bash
./scripts/verify.sh
```

## Use PRYN

```bash
# Show available commands
cargo run -- --help

# Check Wi-Fi, Bluetooth, and audio
cargo run -- check

# Check one area
cargo run -- check wifi
cargo run -- check bluetooth
cargo run -- check audio
```

## Save results as JSON

```bash
cargo run -- check --format json
cargo run -- check --format json --output pryn-report.json
```

PRYN does not overwrite an existing file unless `--force` is supplied:

```bash
cargo run -- check --format json --output pryn-report.json --force
```

## Quality checks

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
cargo build --release --locked
```

## Project principles

- Read-only by default.
- No root requirement for the checks currently available.
- No telemetry or PRYN-initiated network requests. Local system D-Bus is used to inspect supported services. NetworkManager may independently perform its configured connectivity check; PRYN only reads the resulting status.
- Missing information is never reported as success.
- System information collection is kept separate from the rules that interpret it.
- JSON results include a schema version for compatibility.

See [`docs/architecture.md`](docs/architecture.md), [`docs/results.md`](docs/results.md), and [`docs/support-matrix.md`](docs/support-matrix.md).
