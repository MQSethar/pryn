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

- Wi-Fi: `/sys/class/net`, wireless interface markers, `operstate`, `carrier`, driver symlinks, and best-effort `/sys/class/rfkill` attributes.
- Bluetooth: `/sys/class/bluetooth` controller entries, driver symlinks, and best-effort `/sys/class/rfkill` attributes.
- Audio: `/sys/class/sound` ALSA card entries and driver symlinks.
- System metadata: `/etc/os-release` and `/proc/sys/kernel/osrelease`.

The collectors are read-only. They do not execute system commands, access network services, change radio state, restart services, or request root privileges. The current rfkill sysfs reads are best-effort and are not an atomic snapshot. The kernel recommends `/dev/rfkill` for a consistent userspace snapshot; a dedicated safe reader is a planned follow-up before expanding radio diagnostics.

## Important boundaries

- A network interface carrier is not proof of IP configuration, routing, DNS, or internet access.
- A Bluetooth HCI controller entry is not proof that BlueZ is running or that a device is connected.
- An ALSA card entry is not proof that PipeWire/PulseAudio routing or physical audio playback works.
- Missing interfaces and unavailable attributes produce `unknown`, never a fabricated success.
- Unsupported checks are explicitly marked `not_implemented`; subsystem status becomes `partial` when supported checks otherwise complete.

## Expansion rules

Add providers behind platform-facing collector functions. Keep diagnostic rule evaluation pure and test it with synthetic observations. Add service-specific adapters (for example, NetworkManager or BlueZ over D-Bus) only when their contracts and failure modes are tested. Avoid distribution-name branching where capability detection is sufficient.
