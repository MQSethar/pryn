# Architecture

PRYN is currently a single Linux executable with a modular internal structure.
It is intentionally not split into services or dynamically loaded plugins.

## Data flow

1. The CLI parses a requested subsystem and output format.
2. The diagnostic engine creates a read-only Linux platform context.
3. Each selected subsystem module gathers observations from Linux interfaces.
4. Modules convert observations into typed check results.
5. The report model aggregates subsystem status and records a versioned schema.
6. A renderer emits human-readable text or JSON.

## Module boundaries

- cli: command-line parsing and command dispatch.
- diagnostics: orchestration of built-in modules.
- model: stable report and result types.
- modules: Wi-Fi, Bluetooth, and audio diagnostic rules.
- platform/linux: read-only access to sysfs, procfs, and OS metadata.
- output: human-readable and JSON serialization.

Diagnostic rules must not directly execute shell commands, write system files,
or print output. Platform collectors must not decide user-facing severity.
The output layer must not perform system inspection.

## Linux access strategy

The first implementation uses /sys and /proc for kernel-exposed state.
Provider integrations such as NetworkManager, iwd, BlueZ, PipeWire, and
PulseAudio will be added behind subsystem-specific collection boundaries.
PRYN must detect available capabilities instead of assuming a particular
Linux distribution or desktop stack.

## Safety boundaries

- Normal diagnosis is read-only.
- No root escalation is requested by the application.
- No automatic repairs are performed.
- No network credentials, SSIDs, or MAC addresses are collected by the
  initial collectors.
- Missing or inaccessible data is represented as unknown where evidence
  is insufficient; it must not be converted into a passing result.

## Extending a subsystem

Add provider collection to the relevant platform/linux boundary, normalize
its observations, and consume those observations from the subsystem module.
Add fixture-based tests for both successful and unavailable provider states.
Preserve existing diagnostic codes when their meaning has not changed. Treat
report schema changes as compatibility changes and update schema_version
only with an intentional documented migration.
