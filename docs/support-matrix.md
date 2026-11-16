# Support matrix

PRYN targets Linux x86_64. The v0.1.1 prebuilt executable is statically linked for `x86_64-unknown-linux-musl`; native `.deb` and `.rpm` packages reuse the same executable. Service-level diagnostics depend on what is available on the host and in the current user session.

## Release and architecture

| Target | Availability | Notes |
|---|---|---|
| Linux x86_64 / amd64 | Available in v0.1.1 | Static musl executable, `.deb`, `.rpm`, and generic archive |
| Linux AArch64 / ARM64 | Not supported | Deferred until there is demand and a test device |

## Automated container smoke tests

| Environment | Package or executable tested | Result | What was verified |
|---|---|---|---|
| Ubuntu 26.04 | `.deb` | Passed | Install, `--version`, `--help`, process startup, JSON report |
| Debian 13 | `.deb` | Passed | Install, `--version`, `--help`, process startup, JSON report |
| Kali Rolling | `.deb` | Passed | Install, `--version`, `--help`, process startup, JSON report |
| Fedora latest | `.rpm` | Passed | Install, `--version`, `--help`, process startup, JSON report |
| Alpine latest | Static musl archive | Passed | Extract, `--version`, `--help`, process startup, JSON report |

A container smoke test is not physical hardware validation and does not prove all subsystem providers are present.

## Real-machine validation

| Environment | Architecture | Status | Notes |
|---|---|---|---|
| Arch Linux | x86_64 | Runtime tested | Maintainer's real machine |
| Ubuntu 26.04 | x86_64 | Runtime tested | Real machine; diagnostics exercised with the host services present |
| Kali Rolling | x86_64 | Runtime tested | Real machine; missing BlueZ was reported as unavailable/unknown |
| Debian 13 | x86_64 | Not individually machine-tested | CI container smoke test only |
| Fedora | x86_64 | Not individually machine-tested | CI container smoke test only |
| openSUSE | x86_64 | Not validated | RPM is intended for local installation |
| Alpine | x86_64 | Not individually machine-tested | CI container startup smoke test only |
| Linux Mint / Pop!_OS / other derivatives | x86_64 | Not individually validated | Expected to be compatible with the relevant base-family package; test before making a support claim |

## Package formats

| Artifact | Ecosystem | Architecture | Shared runtime dependency |
|---|---|---|---|
| `.deb` | Debian / Ubuntu / Kali family | amd64 | None declared; contains static musl executable |
| `.rpm` | Fedora / RHEL family / openSUSE (intended) | x86_64 | None declared; contains static musl executable |
| Generic tarball | Compatible x86_64 Linux | x86_64 | None expected; static-musl binary |
| `pryn-bin` PKGBUILD | Arch local build / future AUR package | x86_64 | No Rust/Cargo; builds a package around the prebuilt archive |
| crates.io | Rust/Cargo users | Host target | Rust/Cargo required to compile |

The `pryn-bin` recipe is attached to the release but has not yet been submitted to the AUR. `yay -S pryn-bin` is not available until that submission exists.

## Runtime capabilities

- Wi-Fi manager state: NetworkManager or standalone iwd over system D-Bus.
- Bluetooth adapter and device state: BlueZ over system D-Bus.
- Audio endpoint details: ALSA discovery and a PulseAudio-compatible user socket.
- Native PipeWire graph details: optional `pw-dump`.
- IP addresses and routes: optional `ip` from iproute2.
- DNS configuration: `/etc/resolv.conf`.

PRYN does not install or manage any of these services. A missing service is an unavailable diagnostic capability, not a package installation failure.
