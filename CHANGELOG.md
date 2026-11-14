# Changelog

## [0.1.0] - Unreleased

- Replace the placeholder binary with a structured `check` command.
- Add shared report models and versioned JSON output.
- Add read-only Linux sysfs discovery for Wi-Fi, Bluetooth, and ALSA sound cards.
- Add best-effort rfkill checks for Wi-Fi and Bluetooth.
- Add evidence-backed results for blocked radios and absent Wi-Fi carrier.
- Add synthetic collector/rule tests and CLI integration tests.
- Explicitly mark service-level checks that are not implemented yet.

- Add read-only NetworkManager D-Bus collection with bounded method timeouts.
- Check global networking, Wi-Fi software/hardware radio state, managed adapters, connection state, and firmware/plugin availability signals.
- Add read-only standalone iwd Device/Station inspection through its system-bus ObjectManager when NetworkManager is absent; keep NetworkManager as the manager-facing API when present.
- Add synthetic tests for active, disconnected, unavailable, failed, unmanaged, and software-disabled NetworkManager Wi-Fi states, plus connected/disconnected/transitional standalone iwd Station states.
- Read NetworkManager connectivity only when its configured check is enabled and available; label it as system-wide rather than Wi-Fi-specific and use network-level finding identifiers.
- Avoid duplicate generic-unavailable and missing-firmware findings.
- Add read-only audio-server inspection using a pure-Rust PulseAudio protocol client, supporting native PulseAudio and PipeWire's `pipewire-pulse` compatibility service.
- Inspect default playback/capture endpoints, mute and channel-volume state, formats, ports, and active card profiles.
- Exclude monitor sources from non-monitor capture-source counts and treat suspended endpoints as normal idle state.
- Keep audio hardware discovery independent from audio-server availability; do not play samples or record microphone audio during checks.

- Replace the asynchronous PulseAudio client reactor with bounded blocking protocol queries, avoiding a detached reactor thread after timeout.
- Add native PipeWire graph inspection through `pw-dump` JSON with a three-second process timeout and an 8 MiB output cap.
- Support native PipeWire endpoint/default metadata as a fallback when the PulseAudio-compatible socket is unavailable.
- Keep mute state unknown when a provider does not expose it; do not fabricate an unmuted result.
- Give zero capture level its own stable finding code (`AUDIO-012`) rather than reusing the microphone-mute code.

- Add read-only BlueZ system D-Bus inspection for controller power, discovery/visibility, paired devices, trust state, blocked state, and active connections.
- Keep Bluetooth controller inventory, rfkill, BlueZ service availability, adapter power, and device connection as independent diagnostic checks.
- Add Bluetooth audio-card/profile diagnostics for PipeWire `bluez5` and PulseAudio-compatible BlueZ cards; redact MAC-like Bluetooth endpoint identifiers from reports.
- Avoid false `AUDIO-008` findings for Bluetooth cards left on the `off` profile after a device disconnects, redact Bluetooth identifiers when a default endpoint cannot be matched to inventory, and preserve unknown when the active profile availability conflicts with provider state.
- Add synthetic tests for powered/off adapters, connected and disconnected devices, unavailable BlueZ, and Bluetooth audio profile visibility. Treat active-profile/availability conflicts as unknown rather than pass or hardware fault.

- Add read-only Wi-Fi IP-address, IPv4/IPv6 default-route, and DNS-configuration diagnostics with bounded `ip -j` collection.
- Add concise human output by default and `--verbose` for full checks/evidence.
- Define CLI exit codes: 0 for a clean report, 1 for diagnostic attention/incomplete results, and 2 for execution/output errors.
- Normalize PulseAudio channel volumes against `PA_VOLUME_NORM` rather than applying the cubic acoustic-gain conversion to configured sink/source levels.
- Filter auxiliary Bluetooth sysfs entries such as `hci0:51` from canonical HCI controller inventory.
- Consolidate CI verification through `scripts/verify.sh` using the pinned Rust toolchain.

- Add dual MIT OR Apache-2.0 licensing and crates.io package metadata.
- Add a release verification step that builds the distributable Cargo package.
- Add reproducible Linux x86_64 GNU binary packaging with SHA-256 checksums.
- Add a tag-triggered GitHub Actions workflow that creates a draft release for manual review.
- Document first-release, crates.io, binary installation, and security-reporting procedures.
