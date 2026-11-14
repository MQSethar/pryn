# PRYN

**Simple system checks for Linux.**

PRYN checks Wi-Fi, Bluetooth, and audio. It shows what it can confirm, explains what may be wrong, and suggests safe next steps. Checks are read-only: PRYN does not change device settings, restart services, play test audio, or record microphone audio.

> **Development status:** first read-only CLI MVP. PRYN inspects Wi-Fi hardware and either NetworkManager or standalone iwd state, global IP addresses on wireless interfaces, system default routes, resolver configuration, BlueZ adapter/device state, and ALSA/PulseAudio/PipeWire audio configuration. Final release validation still requires running the verification suite and exercising the iwd and audio paths on real hardware.

## What works today

- Finds wireless network interfaces and checks their link state.
- Checks whether Wi-Fi or Bluetooth radios are blocked.
- Reads NetworkManager over the system D-Bus without changing settings.
- Reads NetworkManager's last connectivity result only when its check is configured and enabled; the result is system-wide, may be stale, and is not Wi-Fi-specific.
- Checks NetworkManager's global networking state, Wi-Fi software/hardware switches, managed adapter state, and selected firmware/plugin signals. With standalone iwd, checks the per-device power and Station.State properties that iwd exposes.
- Inspects global IPv4/IPv6 addresses on discovered Wi-Fi interfaces, system IPv4/IPv6 default routes, and configured DNS nameservers. Address and route inspection uses the optional `ip` command from iproute2; DNS configuration is read from `/etc/resolv.conf`. These checks confirm configuration, not internet reachability.
- Reads standalone iwd Device and Station properties over the system D-Bus ObjectManager when NetworkManager is not running; reports interface power and station connection state without scanning, connecting, or exposing network names.
- Finds Bluetooth controllers and ALSA sound cards.
- Reads BlueZ's system D-Bus ObjectManager snapshot to inspect adapter power, discovery/visibility state, and paired, trusted, blocked, and connected device state.
- Distinguishes controller detection, BlueZ service availability, adapter power, device pairing, and active connections instead of treating them as one Boolean.
- Does not initiate Bluetooth discovery, pairing, connection, or configuration changes; device addresses are not included in diagnostic output.
- Inspects native PulseAudio and PipeWire's PulseAudio-compatible service over the user-session Unix socket using a pure-Rust protocol implementation.
- Inspects native PipeWire Core, audio Nodes, Devices, and default routing metadata from the documented `pw-dump` JSON output when `pw-dump` is installed.
- Checks playback and non-monitor capture endpoints, default endpoint selection, mute state, channel volume, sample format, sample rate, and channel count.
- Inspects available card profiles and selected ports when the active audio provider exposes them, including Bluetooth audio cards and their active profiles. Conflicting provider data (for example, an active profile simultaneously marked unavailable) remains unresolved instead of being reported as healthy or broken.
- Treats suspended audio endpoints as normally idle, not as failed devices.
- Excludes playback monitor sources from capture-device counts. Other sources are not automatically assumed to be physical microphones.
- Shows a concise terminal summary by default, includes all checks and evidence with `--verbose`, or saves results as schema-versioned JSON.
- Uses exit code `0` for clean completed checks, `1` when a report contains warnings/problems/unknown or incomplete checks, and `2` for execution/output errors.
- Runs without root privileges. PRYN does not send test audio, record audio, or enumerate application stream names during `pryn check audio`.

## MVP boundaries

- NetworkManager is preferred when available. Standalone iwd is also inspected through its read-only Device and Station D-Bus interfaces; NetworkManager-managed iwd continues to use NetworkManager as the manager-facing API.
- PRYN reads configured IP addresses, default routes, and DNS settings. It does not send DNS queries, ping hosts, or make HTTP requests to prove reachability.
- Audio checks inspect configured endpoints, profiles, mute/volume state, and the PipeWire graph. They do not play test audio or record microphone input, so a passing configuration check does not prove that sound is physically audible or that a microphone captures intelligible audio. When the provider cannot report jack/port availability, PRYN reports `unknown`; it does not guess.
- Bluetooth checks inspect the kernel controller, rfkill, BlueZ adapters/devices, and exposed audio profiles. They do not initiate discovery, pairing, or connections.
- No TUI, background daemon, auto-repair, telemetry, or privileged repair actions are included in the first MVP.

## Requirements for building PRYN

- Linux (x86_64 is the initial development target).
- The Rust version listed in `rust-toolchain.toml`.
- Cargo (installed with Rustup).

## Optional audio runtime tools

- A local PulseAudio-compatible Unix socket is used for endpoint, volume, mute, format, port, and profile details. This works with native PulseAudio and PipeWire's `pipewire-pulse` compatibility service.
- `pw-dump` is optional and provides additional native PipeWire graph inspection through structured JSON. PRYN invokes it directly without a shell, applies a three-second timeout, caps collected output at 8 MiB, and retains only audio-device objects and default routing metadata.
- ALSA hardware discovery continues to work even when neither audio server is available.

## Install PRYN

### From crates.io

After the first release is published:

```bash
cargo install pryn --locked
```

### From a GitHub release

Download the `pryn-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` archive and its `SHA256SUMS` file from [GitHub Releases](https://github.com/MQSethar/pryn/releases). Verify the archive before extracting it:

```bash
sha256sum --check SHA256SUMS
tar -xzf pryn-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz
install -Dm755 pryn-vX.Y.Z/pryn ~/.local/bin/pryn
```

Ensure `~/.local/bin` is in your `PATH`. The first binary release targets x86_64 Linux with GNU libc; other architectures and non-glibc environments are not claimed yet.

### Build from source

```bash
cargo check --locked
cargo build --release --locked
cargo install --path . --locked
```

`cargo install --path .` installs the current checkout into Cargo's user binary directory. It does not publish a package or create a distribution package.

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

# Show every check and supporting evidence
cargo run -- check --verbose
cargo run -- check wifi --verbose
```

## Exit codes

- `0`: completed checks found no warning/problem/unknown state.
- `1`: the report contains a warning, problem, unknown, partial, or not-implemented check. Read the report; this is not necessarily a fatal execution error.
- `2`: PRYN could not execute the command or write the requested report.

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

## License

PRYN is dual-licensed under either the [MIT License](LICENSE-MIT) or [Apache License, Version 2.0](LICENSE-APACHE), at your option.

## Project principles

- Read-only by default; no root requirement for supported checks.
- No telemetry or PRYN-initiated network requests. The optional `ip` command only reads local kernel network state.
- No shell is used for system inspection. Optional system tools such as `ip` and `pw-dump` are invoked directly with bounded execution time and output; their output is parsed as data.
- Missing information is never reported as success.
- System information collection is kept separate from the rules that interpret it.
- JSON results include a schema version for compatibility.

Diagnostic JSON can contain local IP addresses, default gateways, and DNS server addresses. Review it before sharing a report publicly.

See [`docs/architecture.md`](docs/architecture.md), [`docs/results.md`](docs/results.md), [`docs/support-matrix.md`](docs/support-matrix.md), [`docs/audio-diagnostics.md`](docs/audio-diagnostics.md), and [`docs/releasing.md`](docs/releasing.md).
