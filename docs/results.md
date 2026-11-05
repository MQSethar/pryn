# How PRYN reports results

PRYN distinguishes observations from conclusions.

## Result statuses

- `pass`: the check completed and its stated condition was observed.
- `warning`: a potentially relevant condition was observed, but it is not proof of a root cause.
- `problem`: evidence supports a specific failure condition.
- `unknown`: evidence was unavailable or insufficient.
- `not_applicable`: the check does not apply to this system.
- `not_implemented`: the check is planned but is not implemented in this version.

Subsystem/report statuses additionally include `partial`, meaning some checks remain unimplemented.

## Severity

Severity is attached to findings, not used as a substitute for check status. A warning can exist without a high-severity finding. `unknown` is never silently converted to `pass`.

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
- `NET-001`: NetworkManager reports that a captive-portal sign-in may be required.
- `NET-002`: NetworkManager reports limited system connectivity.
- `BT-001`: Bluetooth radio has an rfkill block.

These identifiers are part of the JSON report contract. Renaming or changing their meaning requires a documented compatibility decision.

## Current implementation caveats

- rfkill values are read as separate sysfs attributes and may change during collection; do not treat them as an atomic snapshot. The kernel recommends `/dev/rfkill` for a consistent userspace snapshot. PRYN will move to a dedicated reader before relying on event-driven or time-sensitive radio conclusions.
- Wi-Fi `carrier` and `operstate` are kernel link observations. They do not establish IP configuration, default route, DNS, or internet reachability.
- NetworkManager device state `ACTIVATED` indicates a network connection that may be local or global. It is not a direct internet-reachability test.
- NetworkManager connectivity state is system-wide, not specific to Wi-Fi. PRYN only interprets it when the connectivity-check service is configured and enabled. The property contains the last check result and may be stale; PRYN never invokes NetworkManager’s active connectivity-test method.
- NetworkManager state values are interpreted conservatively. Unmanaged devices are not reported as failed, and a disconnected secondary adapter does not create a no-connection finding when another managed adapter is active.

## Data minimization

This version does not collect MAC addresses, IP addresses, SSIDs, hostnames, passwords, or system journal contents. It performs no telemetry or PRYN-initiated network requests. NetworkManager may independently perform a configured connectivity check; PRYN only reads the service’s last reported result.
