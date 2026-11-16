# PRYN

**Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio.**

PRYN checks what Linux exposes, explains what it can confirm, and suggests safe
next steps. It does not change device settings, restart services, pair
Bluetooth devices, play test audio, or record microphone audio.

## Install

PRYN v0.1.0 is available from crates.io and as an x86_64 GNU/Linux GitHub
release archive. The next distribution release (v0.1.1) adds a static x86_64
musl archive, a Debian-family `.deb`, an RPM-family `.rpm`, and a generated Arch
AUR binary recipe. These packages contain the same executable; there are no
separate Ubuntu/Kali/Debian/Fedora binaries.

### From crates.io

This compiles PRYN and therefore requires Rust and Cargo:

```bash
cargo install pryn --locked
```

### From a GitHub release archive

Download the x86_64 Linux archive and `SHA256SUMS` from
[GitHub Releases](https://github.com/MQSethar/pryn/releases). Verify the
checksum before extracting. The v0.1.0 archive is GNU-linked; the static musl
archive is introduced in v0.1.1.

```bash
sha256sum --check SHA256SUMS
tar -xzf pryn-vVERSION-x86_64-unknown-linux-musl.tar.gz
install -Dm755 pryn-vVERSION/pryn ~/.local/bin/pryn
```

Ensure `~/.local/bin` is in `PATH`.

### Debian, Ubuntu, Kali, and derivatives (v0.1.1+)

Download the `.deb` asset and install it with:

```bash
sudo apt install ./pryn_VERSION_amd64.deb
```

One amd64 package is intended for compatible Debian-family distributions. It
is a direct-download package; a hosted signed APT repository is not provided
in the initial distribution release.

### Fedora, RHEL-compatible, and openSUSE (v0.1.1+)

Download the `.rpm` asset and install it with:

```bash
sudo dnf install --nogpgcheck ./pryn-VERSION-1.x86_64.rpm
```

For openSUSE, use `zypper install ./pryn-VERSION-1.x86_64.rpm`. The initial
RPM is unsigned; the flag is required for local installation until signed
repositories are introduced.

### Arch Linux (v0.1.1+, after AUR submission)

```bash
yay -S pryn-bin
```

`pryn-bin` is a community AUR package, not an official Arch repository package.

## Runtime independence

Prebuilt static musl packages do not require Rust, Cargo, a compiler, glibc,
libgcc, or other shared runtime libraries. They target x86_64 Linux. AArch64
is deferred until there is demand and a test device.

PRYN inspects optional host services; it does not install or manage them:

- NetworkManager or standalone iwd for Wi-Fi manager state;
- BlueZ for Bluetooth state;
- ALSA and a PulseAudio-compatible socket for audio details;
- `pw-dump` for additional native PipeWire graph inspection (optional);
- `ip` from iproute2 for IP address and route inspection (optional).

If a service or optional utility is missing, the related result is unavailable
or unknown. This does not prevent installation or execution. PRYN never
requires root privileges for its supported checks.

## What PRYN checks

- Wireless interfaces, drivers, carrier state, rfkill, NetworkManager or iwd
  state, IP addresses, default routes, and configured DNS servers.
- Bluetooth controllers, rfkill, BlueZ service, adapter power/visibility,
  paired devices, and connection state.
- ALSA cards, PulseAudio/PipeWire server state, playback and capture endpoints,
  defaults, mute/volume/channel state, formats, ports, and exposed profiles.
- Human-readable summaries, detailed evidence with `--verbose`, and
  schema-versioned JSON output.

PRYN does not perform active internet reachability tests. Audio configuration
does not prove sound is physically audible or microphone capture is
intelligible. Bluetooth checks do not perform discovery, pairing, or
connections. Missing provider information is never reported as success.

## Usage

```bash
pryn --help
pryn check
pryn check wifi
pryn check bluetooth
pryn check audio
pryn check --verbose
pryn check --format json
pryn check --format json --output pryn-report.json
```

Exit codes:

- `0`: completed checks found no warning, problem, unknown, or incomplete state.
- `1`: report contains a warning/problem/unknown/incomplete result. Read the report; this is not a process crash.
- `2`: command execution or report-output error.

Diagnostic JSON can contain local IP addresses, gateways, and DNS server
addresses. Review it before sharing a report publicly.

## Build from source

```bash
cargo check --locked
cargo build --release --locked
./scripts/verify.sh
```

The pinned Rust toolchain is specified in `rust-toolchain.toml`.

## Distribution and support

See [`docs/distribution.md`](docs/distribution.md) and
[`docs/support-matrix.md`](docs/support-matrix.md) for package formats,
architecture, host-service requirements, and tested environments.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or
[Apache License 2.0](LICENSE-APACHE), at your option.
