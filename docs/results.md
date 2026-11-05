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
- `WIFI-002`: all discovered Wi-Fi interfaces report no carrier.
- `BT-001`: Bluetooth radio has an rfkill block.

These identifiers are part of the JSON report contract. Renaming or changing their meaning requires a documented compatibility decision.

## Current implementation caveats

- rfkill values are read as separate sysfs attributes and may change during collection; do not treat them as an atomic snapshot. The kernel recommends `/dev/rfkill` for a consistent userspace snapshot. PRYN will move to a dedicated reader before relying on event-driven or time-sensitive radio conclusions.
- Wi-Fi `carrier` and `operstate` are kernel link observations. They do not establish association, IP configuration, default route, DNS, or internet reachability.

## Data minimization

This version does not collect MAC addresses, IP addresses, SSIDs, hostnames, passwords, or system journal contents. It performs no telemetry or network requests.
