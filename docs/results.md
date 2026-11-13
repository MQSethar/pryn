# How PRYN reports results

PRYN distinguishes observations from conclusions.

## Result statuses

- `pass`: the check completed and its stated condition was observed.
- `warning`: a potentially relevant condition was observed, but it is not proof of a root cause.
- `problem`: evidence supports a specific failure condition.
- `unknown`: evidence was unavailable or insufficient.
- `not_applicable`: the check does not apply to this system or provider.
- `not_implemented`: the check is planned but is not implemented in this version.

Subsystem/report statuses additionally include `partial`, meaning some checks remain unimplemented.

## Severity

Severity is attached to findings, not used as a substitute for check status. A warning can exist without a high-severity finding. `unknown` is never silently converted to `pass`.

## CLI process exit codes

- `0`: the completed report contains no warning, problem, unknown, partial, or unimplemented checks. `not_applicable` checks do not make a report fail.
- `1`: the report completed, but contains a warning, problem, unknown, partial, or not-implemented check. Read the report; this is a diagnostic result, not an execution failure.
- `2`: PRYN could not parse/execute the command or could not write the requested report.

## Finding identifiers

Finding codes are intended to remain stable once publicly documented. Current codes:

- `WIFI-001`: Wi-Fi radio has an rfkill block.
- `WIFI-003`: NetworkManager reports that global networking is disabled.
- `WIFI-004`: NetworkManager reports that the Wi-Fi software switch is disabled.
- `WIFI-005`: NetworkManager reports that wireless hardware is disabled.
- `WIFI-006`: a NetworkManager-managed Wi-Fi connection attempt failed.
- `WIFI-007`: NetworkManager reports that a Wi-Fi adapter is unavailable.
- `WIFI-008`: NetworkManager reports that Wi-Fi authentication is required.
- `WIFI-009`: no NetworkManager-managed Wi-Fi adapter is connected.
- `WIFI-010`: NetworkManager reports a possible missing firmware or device plugin dependency.
- `WIFI-011`: a Wi-Fi link is up but no global IPv4 or IPv6 address is configured on a discovered Wi-Fi interface.
- `NET-001`: NetworkManager reports that a captive-portal sign-in may be required.
- `NET-002`: NetworkManager reports limited system connectivity.
- `NET-003`: no DNS nameserver is configured in `/etc/resolv.conf` while a Wi-Fi link is up.
- `NET-004`: no system IPv4 or IPv6 default route is configured while a Wi-Fi link is up.
- `BT-001`: Bluetooth radio has an rfkill block.
- `BT-002`: all BlueZ Bluetooth adapters are powered off.
- `AUDIO-001`: no usable default playback output was confirmed.
- `AUDIO-002`: the default playback output is muted.
- `AUDIO-003`: the default playback volume is zero.
- `AUDIO-004`: the default playback output is amplified above 100%.
- `AUDIO-005`: the selected output port reports unavailable.
- `AUDIO-006`: the default capture input is muted.
- `AUDIO-007`: the selected input port reports unavailable.
- `AUDIO-008`: an audio card is using its `off` profile.
- `AUDIO-009`: one or more playback channels are zero while other channels are active.
- `AUDIO-010`: one or more capture channels are zero while other channels are active.
- `AUDIO-011`: the default capture input is amplified above 100%.
- `AUDIO-012`: the default capture input level is zero.

These identifiers are part of the JSON report contract. Renaming or changing their meaning requires a documented compatibility decision.

## Audio interpretation boundaries

- A suspended endpoint is normally idle and is not reported as broken.
- A mute state may be intentional. Microphone mute is an informational privacy finding.
- PulseAudio channel volumes are normalized against `PA_VOLUME_NORM`; `1.0` means the server-reported 100% reference level and values above `1.0` indicate configured amplification. PRYN does not convert sink/source settings through the cubic acoustic-gain function because PulseAudio documents that conversion as unsuitable unless the device advertises decibel volume. Configuration alone cannot prove clipping.
- Monitor sources capture playback output and are excluded from non-monitor capture-source counts.
- Non-monitor capture sources may be virtual; they are not automatically described as physical microphones.
- A channel at zero while another is active is a balance warning, not proof of hardware damage.
- BlueZ adapter power, pairing, and connection are separate states. A disconnected paired device is not treated as a hardware failure.
- Bluetooth audio profiles are reported separately from BlueZ device connectivity; an `off` profile may be expected when a device is disconnected or not in use and is not raised as a generic `AUDIO-008` failure.
- PipeWire native graph inspection uses the optional `pw-dump` JSON tool. It does not inspect application stream names or prove physical playback/capture.
- PRYN does not play test audio or record microphone audio during `pryn check audio`; a passing configuration check does not prove physical playback or intelligible capture.

## Current implementation caveats

- rfkill values are read as separate sysfs attributes and may change during collection; they are not an atomic snapshot.
- Wi-Fi IP-address inspection uses `ip -j address show` when iproute2 is installed. Only global addresses associated with discovered wireless interfaces are included.
- Default-route inspection reads the system IPv4 and IPv6 default routes. A route through another interface is not classified as a Wi-Fi routing failure.
- DNS inspection reads nameserver entries from `/etc/resolv.conf`. On systems using a local resolver stub, this reports the stub address; it does not inspect per-link resolver state or prove that DNS queries succeed.
- IP addresses, routes, and resolver configuration describe local configuration, not end-to-end connectivity. PRYN does not send ping, DNS, or HTTP probes.
- NetworkManager connectivity state is system-wide, may be stale, and is not a direct per-Wi-Fi reachability test.

## Data minimization

Diagnostic output does not expose MAC addresses, Wi-Fi SSIDs, passwords, journal contents, microphone recordings, application stream names, or media titles. The BlueZ `Address` property is not copied into `BluezSnapshot`, and MAC-like Bluetooth card/endpoint labels are redacted before rendering. The native PipeWire adapter discards Client and Stream objects. The PulseAudio handshake cookie is never logged or serialized. Local IP addresses, default gateways, and DNS server addresses may appear in reports; review exported JSON before sharing it. PRYN performs no telemetry or PRYN-initiated network requests.
