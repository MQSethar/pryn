# Linux distribution and installation strategy

## Goal

Ship one x86_64 executable for Linux rather than compiling a different PRYN
binary for every distribution. Native packages are thin wrappers around the
same executable. PRYN must not require Rust, Cargo, a compiler, glibc, or any
other shared runtime library for normal installation.

The initial portable target is `x86_64-unknown-linux-musl`. This is a static
musl build. It is intended to run on common x86_64 Linux userlands with a
compatible Linux kernel and normal `/proc`, `/sys`, and D-Bus interfaces. It is
not called universally supported until the release smoke tests and physical
runtime tests pass.

## Release artifacts

Each distribution release produces:

- `pryn-vVERSION-x86_64-unknown-linux-musl.tar.gz`: universal x86_64 Linux archive.
- `pryn_VERSION_amd64.deb`: one Debian-family package for amd64.
- `pryn-VERSION-1.x86_64.rpm`: one RPM-family package for x86_64.
- `pryn-bin.PKGBUILD` and `pryn-bin.SRCINFO`: AUR recipe pinned to the exact
  archive checksum.
- `SHA256SUMS`: checksums for the release assets.

The `.deb`, `.rpm`, AUR package, and archive all contain the exact same
statically linked executable. The native packages declare no external runtime
library dependencies. They do not install NetworkManager, iwd, BlueZ,
PulseAudio, PipeWire, ALSA utilities, `iproute2`, or `pw-dump`.

## Installation methods

### Debian, Ubuntu, Kali, and derivatives

Download the matching `.deb` asset and `SHA256SUMS`, verify the checksum, then:

```bash
sudo apt install ./pryn_VERSION_amd64.deb
pryn --version
pryn check
```

The file package works with dpkg/APT on compatible amd64 Debian-family
systems. It does not make `apt install pryn` work from the default repositories.
A hosted APT repository or inclusion in a distribution repository is a
separate future project.

### Fedora, RHEL, Rocky, AlmaLinux, and openSUSE

Download the matching `.rpm` and `SHA256SUMS`, verify the checksum, then:

```bash
sudo dnf install --nogpgcheck ./pryn-VERSION-1.x86_64.rpm
pryn --version
pryn check
```

For openSUSE, use `zypper install ./pryn-VERSION-1.x86_64.rpm`. The release
RPM is initially unsigned, hence `--nogpgcheck` for local installation. A
hosted signed RPM repository is deferred until the adoption milestone.

### Arch Linux

The generated `pryn-bin.PKGBUILD` is submitted to the AUR as `pryn-bin` after
local validation. The intended installation is:

```bash
yay -S pryn-bin
```

AUR is community-maintained and is not an official Arch repository. The AUR
recipe verifies the downloaded release archive using its pinned SHA-256.

### Any compatible x86_64 Linux distribution

Download the musl archive and checksum, verify, extract, and install:

```bash
sha256sum --check SHA256SUMS
tar -xzf pryn-vVERSION-x86_64-unknown-linux-musl.tar.gz
install -Dm755 pryn-vVERSION/pryn ~/.local/bin/pryn
```

No Rust installation is required for the archive or native packages.

### Rust developers

The crates.io package remains available for source installation:

```bash
cargo install pryn --locked
```

This route requires a Rust toolchain and is not the recommended installation
for ordinary users.

## Runtime capabilities are not bundled dependencies

The executable is self-contained at the shared-library level. The things PRYN
inspects are host capabilities and remain optional:

- system D-Bus, NetworkManager or standalone iwd for Wi-Fi manager state;
- system D-Bus and BlueZ for Bluetooth state;
- ALSA device information and a PulseAudio-compatible socket for audio state;
- native PipeWire inspection when `pw-dump` is installed;
- IP address and route inspection when `ip` from iproute2 is installed.

If a service or optional utility is missing, PRYN reports that capability as
unavailable or unknown. Package installation must not enable, install, or
reconfigure any of those services.

## Compatibility validation

The CI container smoke test installs the `.deb` in Ubuntu, Debian, and Kali;
installs the `.rpm` in Fedora; and executes the static archive in Alpine. The
smoke test checks `--version`, `--help`, process startup, and versioned JSON
output. It does not claim that containers have working physical Wi-Fi,
Bluetooth, or audio hardware.

Physical runtime validation recorded by the maintainer:

- Arch Linux x86_64: tested on real hardware.
- Ubuntu 26.04 x86_64: tested on a real machine.
- Kali Rolling x86_64: tested on a real machine.

Update this list only when a real test is completed. Container smoke tests
must be recorded separately from physical hardware validation.

## No per-distribution binary builds

Do not create separate PRYN executables for Ubuntu, Debian, Kali, Mint, Fedora,
Rocky, AlmaLinux, or openSUSE. Build one x86_64 musl executable and wrap it in
Debian/RPM/AUR metadata. The only planned architecture after x86_64 is
AArch64, once demand and a real test device justify it.

## Signing and hosted repositories are deferred

Initial releases use GitHub TLS, SHA-256 checksums, and transparent source
history. Checksums detect corruption but do not authenticate the publisher if
an attacker can replace both the artifact and checksum. Initial release assets
and packages are not cryptographically signed.

Do not operate hosted APT/RPM repositories yet. They require ongoing signing
key protection, repository metadata generation, key rotation, mirrors, and
incident response.
