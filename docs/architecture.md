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

- Wi-Fi: `/sys/class/net`, wireless interface markers, `operstate`, `carrier`, driver symlinks, and best-effort `/sys/class/rfkill` attributes. NetworkManager state is collected separately over the system D-Bus through `platform/linux/network_manager.rs`. PRYN reads global networking/radio switches, Wi-Fi device state, firmware/plugin signals, and the existing connectivity-check state when NetworkManager reports that its connectivity-check service is configured and enabled. `platform/linux/network_config.rs` optionally invokes `ip -j` directly (no shell, two-second timeout, 2 MiB output cap) to inspect addresses on discovered Wi-Fi interfaces and system default routes; it reads configured DNS nameservers from `/etc/resolv.conf`.
- Bluetooth: `/sys/class/bluetooth` controller entries, driver symlinks, and best-effort `/sys/class/rfkill` attributes. `platform/linux/bluez.rs` checks for `org.bluez` on the system D-Bus and reads one ObjectManager snapshot of `Adapter1` and `Device1` properties. It does not initiate discovery or mutate adapter/device properties; explicit Bluetooth `Address` values and D-Bus object paths are not retained in `BluezSnapshot`. Bluetooth card and endpoint labels are sanitized before report rendering.
- Audio hardware: `/sys/class/sound` ALSA card entries and driver symlinks.
- Audio server compatibility provider: `platform/linux/audio_server.rs` uses the PulseAudio client protocol directly over the current user's Unix socket. It supports native PulseAudio and PipeWire's `pipewire-pulse` compatibility service. It returns owned snapshots of server identity, defaults, sinks, sources, volume/mute state, sample formats, ports, and card profiles.
- Native PipeWire provider: `platform/linux/pipewire.rs` invokes `pw-dump --no-colors` directly, without a shell, and parses its documented JSON output. It retains PipeWire Core version, audio Sink/Source/Duplex Nodes, Device profiles, and default-audio metadata. It intentionally discards application Client and Stream objects. The process has a three-second timeout and an 8 MiB output cap.
- System metadata: `/etc/os-release` and `/proc/sys/kernel/osrelease`.

The collectors are read-only. They do not change radio state, start Bluetooth discovery, pair or connect devices, change audio profiles/default devices/volume/mute state, restart services, play test samples, or record microphone audio. NetworkManager and BlueZ are queried through their D-Bus APIs. D-Bus calls have a three-second per-method timeout; unavailable services and failed reads become unknown results rather than terminating the application. PRYN does not call NetworkManager's active connectivity-test method. Its connectivity property is the result of the last check, not a new live probe; it is system-wide and may reflect another interface, such as Ethernet. iwd is detected by its D-Bus name, but direct iwd property inspection is not implemented yet. The current rfkill sysfs reads are best-effort and are not an atomic snapshot. The kernel recommends `/dev/rfkill` for a consistent userspace snapshot; a dedicated safe reader is a planned follow-up before expanding radio diagnostics.

The PulseAudio protocol handshake and inspection are blocking and bounded by Unix-socket timeouts. The implementation uses the protocol crate's low-level blocking message functions rather than its async client reactor, avoiding a detached reactor thread when an inspection times out. Each inspection query shares a five-second total deadline after the handshake. Authentication cookies are read only for the handshake and are never logged or retained in snapshots.

## Important boundaries

- A network interface carrier is not proof of IP configuration, routing, DNS, or internet access.
- NetworkManager's `ACTIVATED` device state means it reports a network connection (local or global); it does not by itself prove internet reachability.
- A NetworkManager-managed device can be intentionally unmanaged or disconnected. PRYN does not treat those states as proof of broken hardware.
- A Bluetooth HCI controller entry is not proof that BlueZ is running or that a device is connected. A paired device is not necessarily connected, and an audio-capable device does not necessarily expose an active audio profile.
- An ALSA card entry or audio-server endpoint is not proof that sound physically reaches a speaker or that a microphone records intelligible audio.
- A suspended audio endpoint is commonly idle and is not, by itself, a fault.
- PulseAudio monitor sources capture playback output and are excluded from non-monitor capture-source counts.
- A non-monitor capture source is not necessarily a physical microphone; virtual capture sources exist.
- PipeWire graph-node existence does not prove that the device is physically connected or that audio is audible/recordable.
- Missing interfaces and unavailable attributes produce `unknown`, never a fabricated success.
- IP address and default-route inspection depends on the optional iproute2 `ip` executable. Resolver configuration is read-only and does not prove DNS server reachability.
- Unsupported checks are explicitly marked `not_implemented`; subsystem status becomes `partial` when supported checks otherwise complete.

## Expansion rules

Add providers behind platform-facing collector functions. Keep diagnostic rule evaluation pure and test it with synthetic observations. The NetworkManager adapter returns plain snapshot types; diagnostic rules do not import zbus or call D-Bus. Audio providers similarly return owned snapshots; rules do not import PulseAudio protocol or JSON process details. Add other service adapters (iwd) behind the same boundary when their contracts and failure modes are tested. BlueZ and NetworkManager adapters return plain snapshots; diagnostic rules do not import zbus or call D-Bus. Avoid distribution-name branching where capability detection is sufficient.
