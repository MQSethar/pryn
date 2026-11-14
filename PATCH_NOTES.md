# PRYN follow-up implementation: finish the v0.1 diagnostic scope

This is an incremental overlay for the already-applied first-MVP patch. It completes two remaining pieces of diagnostic behavior without expanding PRYN into a daemon, repair utility, or interactive sound tester.

## Implementation changes

### Standalone iwd support

- When NetworkManager is absent and `net.connman.iwd` owns the system-bus name, PRYN reads iwd's root ObjectManager snapshot.
- Reads only `Device.Name`, `Device.Powered`, `Device.Mode`, `Station.State`, and `Station.Scanning`.
- Handles station, access-point, and ad-hoc device modes without labeling a non-station device as a disconnected Wi-Fi client.
- Reports connected, disconnected, transitional, missing, and unavailable states conservatively.
- Keeps NetworkManager as the preferred manager-facing API when it is present, including NetworkManager installations that use iwd internally.
- Does not call `Scan`, `Connect`, `Disconnect`, or write any iwd property. It does not read SSIDs, network object names, or hardware addresses.

### Audio profile consistency

- A profile actively selected by the audio server is not automatically considered healthy if the same provider marks it unavailable or unknown.
- A conflicting or incomplete active-profile inventory is reported as `unknown`, not as a hardware failure and not as a pass.
- The Bluetooth-specific profile check retains this behavior while continuing to redact MAC-like card and endpoint identifiers.
- Port availability remains `unknown` when the audio provider cannot determine physical jack availability. Configuration cannot prove audible playback or intelligible capture.

## Documentation

- Updated the support matrix, architecture, audio interpretation, finding-code definitions, README, and changelog.
- Clarified that active internet probes, unattended playback/recording, Bluetooth pairing/connection automation, a TUI, a daemon, and automatic repair remain outside the read-only v0.1 command.

## Verification

The user's previous local run of the preceding patch reported 81 library tests and 5 CLI tests passing, along with formatting, strict Clippy, and a release build. This follow-up adds synthetic tests for standalone iwd states, non-station iwd modes, and conflicting/unknown audio profile availability.

The supplied latest run demonstrates that the local `unknown_check` helper is now in scope and PRYN successfully recompiles and runs the three subsystem commands. It also shows rustfmt differences in `wifi.rs`; this overlay applies the expected formatting changes. This container does not have `cargo`, `rustc`, or `rustfmt`, so the complete updated overlay has **not** been compiled, formatted by rustfmt, linted, tested, or packaged by Cargo here. On the target machine, run `cargo fmt --all` and `./scripts/verify.sh` before committing. Also validate the standalone iwd path on a machine actually running iwd; the supplied logs exercise NetworkManager, not standalone iwd.

## Release boundaries that remain intentional

- No live ping, DNS, or HTTP internet check. NetworkManager's cached connectivity state is system-wide and may be stale; IP, route, and resolver observations only describe local configuration.
- No test-tone playback or microphone recording. A future `pryn test audio` command needs explicit consent and a separate privacy/safety design.
- No Bluetooth discovery, pairing, connection, or profile switching.
- No background daemon, auto-repair, telemetry, or root requirement.
- rfkill is still a best-effort sysfs snapshot, not an atomic `/dev/rfkill` read.
- Other distributions, non-systemd environments, and aarch64 still need runtime validation.

## Release preparation added

- Select dual licensing under MIT OR Apache-2.0 and include both complete license texts.
- Add crates.io metadata and `cargo package --locked` to the verification gate.
- Add a Linux x86_64 GNU release packaging script that embeds the binary, README, changelog, and both license texts and generates SHA-256 checksums.
- Add a tag-triggered workflow that verifies, builds, packages, and creates a draft GitHub Release for manual review.
- Add a first-release guide covering crate-name availability, package inspection, dry-run publication, manual crates.io publication, tag creation, release review, and installation.
- Add a vulnerability-reporting policy and update README installation/licensing instructions.

No external release action, crates.io token, personal access token, or repository secret is required by the release workflow. Actual publication remains a deliberate manual action because a crates.io version is effectively permanent once published.
