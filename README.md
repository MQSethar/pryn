# PRYN

**Simple system checks for Linux.**

PRYN checks Wi-Fi, Bluetooth, and audio. It shows what it can confirm, explains what may be wrong, and suggests safe next steps. Checks are read-only: PRYN does not change device settings, restart services, play test audio, or record microphone audio.

> **Development status:** early version. PRYN can find Wi-Fi adapters, Bluetooth controllers, ALSA sound cards, visible drivers, and radio block state. Wi-Fi has read-only NetworkManager integration for manager availability, radio switches, adapter connection state, and selected firmware/plugin signals. Audio v1 inspects native PulseAudio, PipeWire's PulseAudio-compatible service, and the native PipeWire graph. Bluetooth service/device inspection, direct iwd inspection, and advanced network IP/route/DNS diagnostics remain future work.

## What works today

- Finds wireless network interfaces and checks their link state.
- Checks whether Wi-Fi or Bluetooth radios are blocked.
- Reads NetworkManager over the system D-Bus without changing settings.
- Reads NetworkManager's last connectivity result only when its check is configured and enabled; the result is system-wide, may be stale, and is not Wi-Fi-specific.
- Checks NetworkManager's global networking state, Wi-Fi software/hardware switches, managed adapter state, and selected firmware/plugin signals.
- Detects iwd, but does not yet inspect its connection state directly.
- Finds Bluetooth controllers and ALSA sound cards.
- Inspects native PulseAudio and PipeWire's PulseAudio-compatible service over the user-session Unix socket using a pure-Rust protocol implementation.
- Inspects native PipeWire Core, audio Nodes, Devices, and default routing metadata from the documented `pw-dump` JSON output when `pw-dump` is installed.
- Checks playback and non-monitor capture endpoints, default endpoint selection, mute state, channel volume, sample format, sample rate, and channel count.
- Inspects available card profiles and selected ports when the active audio provider exposes them.
- Treats suspended audio endpoints as normally idle, not as failed devices.
- Excludes playback monitor sources from capture-device counts. Other sources are not automatically assumed to be physical microphones.
- Shows results in the terminal or saves them as schema-versioned JSON.
- Runs without root privileges. PRYN does not send test audio, record audio, or enumerate application stream names during `pryn check audio`.

## What is coming next

- Direct iwd connection-state inspection.
- Bluetooth service and connected-device diagnostics.
- Advanced Wi-Fi IP address, route, DNS, and optional reachability diagnostics.
- Explicit interactive audio playback and microphone-recording tests, with user consent.
- An interactive terminal screen.

## Requirements for building PRYN

- Linux (x86_64 is the initial development target).
- The Rust version listed in `rust-toolchain.toml`.
- Cargo (installed with Rustup).

## Optional audio runtime tools

- A local PulseAudio-compatible Unix socket is used for endpoint, volume, mute, format, port, and profile details. This works with native PulseAudio and PipeWire's `pipewire-pulse` compatibility service.
- `pw-dump` is optional and provides additional native PipeWire graph inspection through structured JSON. PRYN invokes it directly without a shell, applies a three-second timeout, caps collected output at 8 MiB, and retains only audio-device objects and default routing metadata.
- ALSA hardware discovery continues to work even when neither audio server is available.

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

- Read-only by default; no root requirement for supported checks.
- No telemetry or PRYN-initiated network requests.
- No shell is used for system inspection. The native PipeWire adapter invokes `pw-dump` directly and parses its JSON output.
- Missing information is never reported as success.
- System information collection is kept separate from the rules that interpret it.
- JSON results include a schema version for compatibility.

See [`docs/architecture.md`](docs/architecture.md), [`docs/results.md`](docs/results.md), [`docs/support-matrix.md`](docs/support-matrix.md), and [`docs/audio-diagnostics.md`](docs/audio-diagnostics.md).
