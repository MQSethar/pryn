# Linux distribution and installation strategy

## Goal

Ship one x86_64 executable for Linux rather than compiling a different PRYN binary for every distribution. Native packages are thin wrappers around the same executable. Normal users should not need Rust, Cargo, a compiler, glibc, or another shared runtime library to install and run a prebuilt release.

The portable target is `x86_64-unknown-linux-musl`. The release's `.deb`, `.rpm`, generic archive, and generated Arch binary recipe all reuse that same statically linked executable.

## v0.1.1 release artifacts

The [v0.1.1 GitHub Release](https://github.com/MQSethar/pryn/releases/tag/v0.1.1) contains:

- `pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz`: generic x86_64 Linux archive.
- `pryn_0.1.1_amd64.deb`: Debian-family package.
- `pryn-0.1.1-1.x86_64.rpm`: RPM-family package.
- `pryn-bin.PKGBUILD` and `pryn-bin.SRCINFO`: generated Arch AUR recipe metadata.
- `SHA256SUMS`: SHA-256 checksums for the release assets.

The AUR recipe has not yet been submitted to the AUR. The `.deb` and `.rpm` are direct-download packages, not entries in hosted package repositories. The RPM is unsigned.

## Installation methods

See [Install PRYN on Linux](installation.md) for user-facing instructions and exact commands.

- Debian, Ubuntu, Kali and compatible derivatives: install the local `.deb` using APT.
- Fedora and compatible RPM distributions: install the local `.rpm` using DNF; the current RPM is unsigned.
- openSUSE: the same `.rpm` is the intended local package, but openSUSE has not yet been smoke-tested.
- Arch Linux: build a local package from the generated PKGBUILD. `yay -S pryn-bin` becomes available only after the recipe is actually submitted and visible in the AUR.
- Alpine and other compatible x86_64 distributions: extract the static-musl archive and install the executable manually.
- Rust developers: `cargo install pryn --locked` builds from source and requires Rust/Cargo.

There is no hosted APT or RPM repository yet. Users must download the release file directly. `apt install pryn` and `dnf install pryn` from the default repository indexes are not supported.

## Runtime independence and host capabilities

The executable is statically linked for x86_64 Linux and does not require host glibc or shared runtime libraries. This does not mean every Linux service is bundled.

The following are optional host capabilities:

- NetworkManager or standalone iwd over system D-Bus for Wi-Fi manager state.
- BlueZ over system D-Bus for Bluetooth state.
- ALSA discovery and a PulseAudio-compatible user socket for audio details.
- `pw-dump` for additional native PipeWire graph inspection.
- `ip` from iproute2 for address and route observations.
- `/etc/resolv.conf` for configured DNS nameservers.

PRYN does not install, enable, start, restart, or configure these services. Missing providers must remain unavailable/unknown rather than becoming false passes. A successful install does not imply that Wi-Fi, Bluetooth, audio, or internet diagnostics are all available.

## Compatibility validation

### CI container smoke tests

The v0.1.1 release workflow successfully installed or executed PRYN and checked `--version`, `--help`, process startup, and versioned JSON output in:

- Ubuntu 26.04 container using the `.deb`.
- Debian 13 container using the `.deb`.
- Kali Rolling container using the `.deb`.
- Fedora latest container using the `.rpm`.
- Alpine latest container using the static-musl archive.

These tests validate package/startup behavior only. They do not test physical radios, audio devices, service configuration, or network reachability.

### Real-machine validation

The maintainer's documented real-machine tests are:

- Arch Linux x86_64.
- Ubuntu 26.04 x86_64.
- Kali Rolling x86_64.

Do not convert CI container results into a claim of physical runtime support. Update `docs/support-matrix.md` when new real-machine testing is completed.

## Architecture

- x86_64/amd64: current release target.
- AArch64/ARM64: not supported yet. Add it only when demand and a physical test device justify the maintenance cost.

## Signing and repositories

The initial release provides SHA-256 checksums, but no cryptographic signatures. A checksum detects corruption; it does not independently authenticate the publisher if an attacker can replace both the artifact and checksum.

Signed release artifacts and hosted signed APT/RPM repositories are deferred until the adoption milestone in [signing-and-adoption.md](signing-and-adoption.md). Do not describe the current release packages as signed.
