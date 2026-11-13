# Changelog

## Unreleased (0.1.0)

- Replace the placeholder binary with a structured `check` command.
- Add shared report models and versioned JSON output.
- Add read-only Linux sysfs discovery for Wi-Fi, Bluetooth, and ALSA sound cards.
- Add best-effort rfkill checks for Wi-Fi and Bluetooth.
- Add evidence-backed results for blocked radios and absent Wi-Fi carrier.
- Add synthetic collector/rule tests and CLI integration tests.
- Explicitly mark service-level checks that are not implemented yet.

- Add read-only NetworkManager D-Bus collection with bounded method timeouts.
- Check global networking, Wi-Fi software/hardware radio state, managed adapters, connection state, and firmware/plugin availability signals.
- Detect iwd without claiming unsupported direct iwd inspection.
- Add synthetic tests for active, disconnected, unavailable, failed, unmanaged, and software-disabled Wi-Fi states.
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
- Avoid false `AUDIO-008` findings for Bluetooth cards left on the `off` profile after a device disconnects, and redact Bluetooth identifiers when a default endpoint cannot be matched to inventory.
- Add synthetic tests for powered/off adapters, connected and disconnected devices, unavailable BlueZ, and Bluetooth audio profile visibility.
