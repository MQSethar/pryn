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
