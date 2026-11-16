# Support matrix

PRYN targets Linux. The x86_64 musl executable is designed to avoid a runtime
dependency on the host glibc or other shared libraries. Service-level checks
remain capability-based and depend on the service and optional inspection
tools being present in the current user session.

## Real-machine validation

| Environment | Architecture | Status | Notes |
|---|---|---|---|
| Arch Linux | x86_64 | Runtime tested | Wi-Fi, BlueZ and PipeWire inspected on maintainer hardware. |
| Ubuntu 26.04 | x86_64 | Runtime tested | NetworkManager, BlueZ and PipeWire inspected on a real machine. |
| Kali Rolling | x86_64 | Runtime tested | NetworkManager and audio inspected; BlueZ was not running and correctly reported unavailable. |
| Debian 13 | x86_64 | Pending | Package/container smoke test is not physical hardware validation. |
| Fedora | x86_64 | Pending | Package/container smoke test is not physical hardware validation. |
| Alpine | x86_64 | Pending | Static-musl startup smoke test; host service integrations need separate testing. |
| AArch64 Linux | aarch64 | Not supported yet | Deferred until a test device and user demand exist. |

## Distribution artifacts

| Artifact | Ecosystem | Architecture | Runtime shared-library requirement |
|---|---|---|---|
| Static musl tarball | Generic Linux | x86_64 | None expected; verified by ELF inspection and container smoke tests. |
| `.deb` | Debian, Ubuntu, Kali and derivatives | amd64 | None declared; contains the same static musl executable. |
| `.rpm` | Fedora, RHEL-compatible, openSUSE | x86_64 | None declared; contains the same static musl executable. |
| `pryn-bin` | Arch AUR | x86_64 | None; installs the same static musl executable. |
| crates.io source package | Rust/Cargo | Host target | Requires Rust/Cargo to compile. |

## Runtime capability requirements

- Wi-Fi manager state: NetworkManager or standalone iwd over system D-Bus.
- Bluetooth adapter and device state: BlueZ over system D-Bus.
- Audio endpoint details: ALSA discovery and a PulseAudio-compatible user socket; native PipeWire graph details are enhanced by optional `pw-dump`.
- IP addresses and routes: optional `ip` from iproute2; without it, those observations remain unknown.
- DNS configuration: `/etc/resolv.conf`.

PRYN does not install, enable, start, restart, or configure these services. An
absent service is an unavailable diagnostic capability, not a package install
failure. A passing container smoke test confirms executable/package startup,
not working physical radios, speakers, microphones, or internet reachability.
