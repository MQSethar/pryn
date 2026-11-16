# PRYN

**Read-only Linux diagnostics for Wi-Fi, Bluetooth, and audio.**

PRYN checks what Linux exposes, explains what it can confirm, and suggests safe next steps. It does not change device settings, restart services, pair Bluetooth devices, play test audio, or record microphone audio.

[![Latest release](https://img.shields.io/github/v/release/MQSethar/pryn?label=release)](https://github.com/MQSethar/pryn/releases/latest)
[![CI](https://github.com/MQSethar/pryn/actions/workflows/ci.yml/badge.svg)](https://github.com/MQSethar/pryn/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pryn.svg)](https://crates.io/crates/pryn)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

## Install PRYN

**Latest release: v0.1.1 · Linux x86_64 · no Rust or Cargo required for prebuilt downloads.**

Choose the package for your distribution from the [release downloads](https://github.com/MQSethar/pryn/releases/latest), or follow the [step-by-step installation guide](docs/installation.md).

| Distribution family | Download | Install |
|---|---|---|
| Debian, Ubuntu, Kali and compatible derivatives | [`.deb` for amd64](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn_0.1.1_amd64.deb) | `sudo apt install ./pryn_0.1.1_amd64.deb` |
| Fedora and compatible RPM systems | [`.rpm` for x86_64](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-0.1.1-1.x86_64.rpm) | `sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm` |
| openSUSE | [`.rpm` for x86_64](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-0.1.1-1.x86_64.rpm) | `sudo zypper install --allow-unsigned-rpm ./pryn-0.1.1-1.x86_64.rpm` |
| Arch Linux | [AUR recipe](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-bin.PKGBUILD) | Build locally with `makepkg -si`; AUR publication is pending |
| Alpine and other compatible x86_64 Linux | [Static musl archive](https://github.com/MQSethar/pryn/releases/download/v0.1.1/pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz) | Extract and install the `pryn` executable |

These are direct-download release assets, not hosted APT or RPM repositories. The AUR recipe is generated, but `pryn-bin` has not yet been published to the AUR. Do not use `yay -S pryn-bin` until the package appears in the [AUR package index](https://aur.archlinux.org/packages/?K=pryn-bin).

All prebuilt v0.1.1 packages use the same statically linked x86_64 musl executable. They do not require Rust, Cargo, a compiler, glibc, libgcc, or other shared runtime libraries. ARM64 is not supported yet.

### Verify a download

The release includes `SHA256SUMS`. Download it alongside the asset, then verify the files you downloaded:

```bash
curl -fLO https://github.com/MQSethar/pryn/releases/download/v0.1.1/SHA256SUMS
sha256sum --ignore-missing --check SHA256SUMS
```

The current packages and archive are not cryptographically signed. A SHA-256 checksum detects accidental corruption, but a checksum downloaded from the same release is not a publisher signature.

### Run PRYN

```bash
pryn --version
pryn --help
pryn check
pryn check wifi
pryn check bluetooth
pryn check audio
pryn check --verbose
pryn check --format json
```

PRYN does not need root privileges for its supported checks.

## What PRYN checks

- **Wi-Fi:** interfaces, drivers, carrier, rfkill, NetworkManager or standalone iwd state, IP addresses, default routes, and configured DNS servers.
- **Bluetooth:** controllers, rfkill, BlueZ availability, adapter state, paired devices, and connection state.
- **Audio:** ALSA cards, PulseAudio-compatible server state, PipeWire graph details when `pw-dump` is installed, playback/capture endpoints, defaults, mute/volume/channel state, formats, ports, and exposed profiles.

PRYN inspects optional host services; it does not install or manage them. If a service or optional utility is missing, the related result is unavailable or unknown. A configuration check cannot prove that sound is physically audible, a microphone records intelligible audio, or the internet is reachable.

## Exit codes

- `0`: checks completed without warning, problem, unknown, partial, or unimplemented results.
- `1`: the report completed but contains a warning, problem, unknown, partial, or unimplemented result. Read the report; this is not a process crash.
- `2`: command execution or report-output error.

Diagnostic JSON can contain local IP addresses, gateways, and DNS server addresses. Review it before sharing a report publicly.

## Build from source

This route requires Rust and Cargo:

```bash
cargo install pryn --locked
```

For development:

```bash
cargo check --locked
cargo build --release --locked
./scripts/verify.sh
```

The pinned Rust toolchain is specified in `rust-toolchain.toml`.

## Compatibility and documentation

- [Installation/download website source](site/)
- [Website deployment notes](docs/website.md)
- [Installation guide](docs/installation.md)
- [Distribution strategy](docs/distribution.md)
- [Support matrix](docs/support-matrix.md)
- [Architecture](docs/architecture.md)
- [How results are reported](docs/results.md)
- [Audio diagnostics](docs/audio-diagnostics.md)
- [Release checklist](docs/distribution-release-checklist.md)
- [Security policy](SECURITY.md)

Automated package smoke tests currently cover Ubuntu 26.04, Debian 13, Kali Rolling, Fedora latest, and Alpine latest in containers. Real-machine validation is recorded separately in the support matrix. A container passing does not prove that a distribution's Wi-Fi, Bluetooth, or audio hardware integrations have been tested.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). PRYN is read-only by design: missing or contradictory evidence must remain unknown rather than being presented as success.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE), at your option.
