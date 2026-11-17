# PRYN

**Read-only Linux diagnostics for Wi-Fi, Bluetooth and audio.**

PRYN checks what Linux exposes, explains what it can confirm, and suggests safe next steps. It does not change device settings, restart services, pair Bluetooth devices, play test audio or record microphone audio.

[![Latest release](https://img.shields.io/github/v/release/MQSethar/pryn?label=release)](https://github.com/MQSethar/pryn/releases/latest)
[![CI](https://github.com/MQSethar/pryn/actions/workflows/ci.yml/badge.svg)](https://github.com/MQSethar/pryn/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/pryn.svg)](https://crates.io/crates/pryn)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

## Install

**Prebuilt release: v0.1.1 · Linux x86_64 · no Rust or Cargo required.**

**[Open the PRYN installation website](https://mqsethar.github.io/pryn/)** for the distro selector, direct downloads and the exact install command for each package.

The [step-by-step installation guide](docs/installation.md) is available in the repository. All release assets are on [GitHub Releases](https://github.com/MQSethar/pryn/releases/latest).

| Linux family | Package | Current installation path |
|---|---|---|
| Debian / Ubuntu / Kali | `.deb` | Download, then `sudo apt install ./pryn_0.1.1_amd64.deb` |
| Fedora / RPM | `.rpm` | Download, then `sudo dnf install --nogpgcheck ./pryn-0.1.1-1.x86_64.rpm` |
| Arch | PKGBUILD | Download the recipe, then build with `makepkg -si`; AUR submission is pending |
| Alpine / other compatible x86_64 Linux | Static musl archive | Extract and install the executable into `~/.local/bin` |

The release packages contain the same statically linked x86_64 musl executable. ARM64/aarch64 is not supported yet. Direct-download `.deb` and `.rpm` files are not hosted package repositories; the RPM is unsigned. SHA-256 checksums are provided, but they are not publisher signatures.

## What PRYN checks

- **Wi-Fi:** interfaces, drivers, radio state, NetworkManager or standalone iwd, IP addresses, default routes and configured DNS servers.
- **Bluetooth:** controllers, rfkill, BlueZ availability, adapter power, paired devices and connection state.
- **Audio:** ALSA cards, PulseAudio-compatible server state, optional PipeWire graph details, endpoints, defaults, mute, volume, ports and profiles.

PRYN inspects optional host services; it does not install or manage them. Missing or contradictory information remains unavailable or unknown. A configuration check cannot prove that sound is physically audible, a microphone records intelligible audio, or the internet is reachable.

## Run it

```bash
pryn --version
pryn check
pryn check wifi
pryn check bluetooth
pryn check audio
pryn check --verbose
pryn check --format json
```

PRYN does not need root privileges for its supported checks.

Exit codes: `0` means the completed report has no warning/problem/unknown/partial result; `1` means the report contains one of those diagnostic states; `2` means command execution or report output failed. Exit code `1` is a completed diagnostic report, not necessarily a crash.

## Documentation

- [Installation guide](docs/installation.md)
- [Support matrix](docs/support-matrix.md)
- [How results are reported](docs/results.md)
- [Architecture](docs/architecture.md)
- [Audio diagnostics](docs/audio-diagnostics.md)
- [Distribution strategy](docs/distribution.md)
- [Website maintenance](docs/website.md)
- [Security policy](SECURITY.md)
- [Contributing](CONTRIBUTING.md)

Automated package smoke tests currently cover Ubuntu 26.04, Debian 13, Kali Rolling, Fedora latest and Alpine latest in containers. Real-machine validation is recorded separately in the support matrix. A container passing does not mean the distribution's Wi-Fi, Bluetooth or audio hardware has been tested.

## Build from source

This route requires Rust and Cargo:

```bash
cargo install pryn --locked
```

For development, see `CONTRIBUTING.md` and the pinned toolchain in `rust-toolchain.toml`.

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE), at your option.
