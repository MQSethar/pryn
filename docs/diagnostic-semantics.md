# Diagnostic semantics

## Check status

- pass: the check ran and the inspected condition met its expectation.
- warning: evidence suggests a degraded or potentially problematic state.
- problem: evidence establishes the condition described by the check.
- unknown: the check could not establish the state from available evidence.
- not_applicable: the check does not apply to this host or configuration.

An unavailable provider, missing permission, or unreadable interface must not
be reported as pass or automatically converted into a hardware failure.

## Severity

Severity (info, low, medium, high, critical) is independent of status.
A warning can be medium severity; an unknown result is normally informational.

## Stable diagnostic codes

Diagnostic codes are machine-readable identifiers. Do not change the meaning
of a published code. If the meaning materially changes, introduce a new code
and document the old one as deprecated.

## Current initial checks

| Code | What is inspected |
|---|---|
| WIFI.HARDWARE | Kernel-exposed wireless interfaces and PHY entries |
| WIFI.RADIO | Wi-Fi rfkill soft/hard block state |
| WIFI.CONNECTION | Reserved for connection-manager integration; currently unknown |
| BLUETOOTH.HARDWARE | Kernel-exposed Bluetooth controllers |
| BLUETOOTH.RADIO | Bluetooth rfkill soft/hard block state |
| BLUETOOTH.BLUEZ | Reserved for BlueZ D-Bus integration; currently unknown |
| AUDIO.ALSA_CARDS | ALSA sound-card inventory in /proc/asound/cards |
| AUDIO.SERVER | Reserved for PipeWire/PulseAudio provider integration; currently unknown |

## Exit behavior

A successful diagnostic invocation exits successfully even if it finds warnings
or problems. The command succeeded; the findings describe the machine. A future
explicit automation flag may provide a non-zero exit status based on selected
finding thresholds.
