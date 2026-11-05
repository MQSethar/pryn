# PRYN architecture (v0.1)

PRYN is a Linux-only, read-only system-checking CLI. The first implementation is a modular monolith: one executable, one process, and shared diagnostic models.

## Dependency direction

- `cli` parses arguments and delegates to `app`.
- `app` coordinates subsystem modules and creates a report.
- `modules/*` turn collected observations into checks and findings.
- `platform/linux` reads Linux interfaces and returns typed observations.
- `model` contains serializable domain types and status aggregation.
- `report` renders the same report as human-readable text or JSON.

Diagnostic rules must not read `/sys`, `/proc`, invoke commands, or call D-Bus directly. Platform collectors must not decide whether an observation is a fault. Report renderers must not perform system collection.

## Current collection surface

- Wi-Fi: `/sys/class/net`, wireless interface markers, `operstate`, `carrier`, driver symlinks, and best-effort `/sys/class/rfkill` attributes. NetworkManager state is collected separately over the system D-Bus through `platform/linux/network_manager.rs`. PRYN reads global networking/radio switches, Wi-Fi device state, firmware/plugin signals, and the existing connectivity-check state when NetworkManager reports that its connectivity-check service is configured and enabled.
- Bluetooth: `/sys/class/bluetooth` controller entries, driver symlinks, and best-effort `/sys/class/rfkill` attributes.
- Audio: `/sys/class/sound` ALSA card entries and driver symlinks.
- System metadata: `/etc/os-release` and `/proc/sys/kernel/osrelease`.

The collectors are read-only. They do not execute system commands, make remote network requests, change radio state, restart services, or request root privileges. NetworkManager is queried through its documented D-Bus API. The D-Bus calls have a three-second per-method timeout; unavailable services and failed reads become unknown results rather than terminating the application. PRYN does not call NetworkManager’s active connectivity-test method. Its connectivity property is the result of the last check, not a new live probe; it is system-wide and may reflect another interface, such as Ethernet. iwd is detected by its D-Bus name, but direct iwd property inspection is not implemented yet. The current rfkill sysfs reads are best-effort and are not an atomic snapshot. The kernel recommends `/dev/rfkill` for a consistent userspace snapshot; a dedicated safe reader is a planned follow-up before expanding radio diagnostics.

## Important boundaries

- A network interface carrier is not proof of IP configuration, routing, DNS, or internet access.
- NetworkManager's `ACTIVATED` device state means it reports a network connection (local or global); it does not by itself prove internet reachability.
- A NetworkManager-managed device can be intentionally unmanaged or disconnected. PRYN does not treat those states as proof of broken hardware.
- A Bluetooth HCI controller entry is not proof that BlueZ is running or that a device is connected.
- An ALSA card entry is not proof that PipeWire/PulseAudio routing or physical audio playback works.
- Missing interfaces and unavailable attributes produce `unknown`, never a fabricated success.
- Unsupported checks are explicitly marked `not_implemented`; subsystem status becomes `partial` when supported checks otherwise complete.

## Expansion rules

Add providers behind platform-facing collector functions. Keep diagnostic rule evaluation pure and test it with synthetic observations. The first NetworkManager adapter returns plain snapshot types; diagnostic rules do not import zbus or call D-Bus. Add other service adapters (iwd, BlueZ, PipeWire) behind the same boundary when their contracts and failure modes are tested. Avoid distribution-name branching where capability detection is sufficient.
