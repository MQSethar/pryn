# PRYN first-MVP implementation patch

This overlay completes the scoped first CLI MVP for Wi-Fi, Bluetooth, and audio. It deliberately does not add a TUI, daemon, auto-repair, telemetry, direct iwd support, active internet probes, or unattended audio playback/recording tests.

## Correctness fixes

1. Annotates the BlueZ `NameHasOwner` D-Bus return as `bool`, resolving the reported `E0282` inference failure.
2. Filters `/sys/class/bluetooth` to canonical `hci<number>` controller entries, excluding auxiliary entries such as `hci0:51`.
3. Normalizes PulseAudio sink/source levels against `PA_VOLUME_NORM`; a configured 5% remains 5% instead of being cubic-converted to a near-zero display value.
4. Uses carrier state ahead of a stale `operstate=up` when deciding whether a Wi-Fi link is currently present.
5. Preserves `unknown` when the Wi-Fi interface inventory is unavailable instead of inventing negative address/route conclusions.
6. Keeps IP addresses, default routes, DNS configuration, and NetworkManager's last connectivity result as separate observations. None is presented as proof of end-to-end internet reachability.
7. Keeps Bluetooth controller inventory, BlueZ service state, adapter power, device pairing/connection, and Bluetooth audio profiles as separate checks. Device addresses are not retained in the BlueZ diagnostic snapshot.

## First-MVP functionality

- Read-only Wi-Fi IP address inspection for discovered wireless interfaces.
- IPv4 and IPv6 system default-route inspection.
- Read-only nameserver configuration inspection from `/etc/resolv.conf`.
- Optional `iproute2` execution through `ip -j`, without a shell, with a 2-second timeout and 2 MiB output cap.
- Compact human-readable output by default; `pryn check --verbose` includes every check and its evidence.
- JSON output remains schema-versioned and includes full diagnostic evidence.
- Exit codes: `0` clean completed report, `1` findings or unresolved/incomplete diagnostics, `2` execution/output failure.
- CI uses `actions/checkout@v7` and `actions-rust-lang/setup-rust-toolchain@v2` to install the toolchain/components specified by `rust-toolchain.toml`, then runs the same `scripts/verify.sh` used locally.
- Cargo package metadata includes repository, README, and discovery keywords. No license is selected on the user's behalf.

## Verification status

- Source and documentation were reviewed in this environment.
- ZIP integrity is checked after packaging.
- This environment does not have `cargo`, `rustc`, or `rustfmt`; therefore this patch has **not** been compiled, formatted by rustfmt, linted, or tested here. Run `cargo fmt --all` followed by `./scripts/verify.sh` on the pinned Rust toolchain before committing or merging.

## Runtime boundaries

- NetworkManager is the supported Wi-Fi connection-manager provider. iwd is identified but direct iwd connection-state inspection is outside this MVP.
- `/etc/resolv.conf` may point to a local stub and is not proof that DNS resolution works.
- A route through a different interface is not classified as a Wi-Fi route failure.
- Audio endpoint/profile configuration does not prove physical playback or intelligible microphone capture. PRYN does not play or record audio automatically.
- Bluetooth discovery, pairing, connecting, profile switching, and radio changes remain outside the read-only diagnostic command.
