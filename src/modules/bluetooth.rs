use crate::model::{
    DiagnosticCheck, DiagnosticStatus, Evidence, Finding, Severity, Subsystem, SubsystemReport,
};
use crate::platform::linux::bluez::{BluezAdapter, BluezProbe};
use crate::platform::linux::{BluetoothSnapshot, LinuxCollector};

use super::rfkill::evaluate_radio_block;

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    let hardware = collector.collect_bluetooth();
    let bluez = crate::platform::linux::bluez::collect();
    evaluate_snapshot(hardware, bluez)
}

fn evaluate_snapshot(
    hardware: Result<BluetoothSnapshot, crate::platform::linux::ProbeError>,
    bluez: BluezProbe,
) -> SubsystemReport {
    let mut checks = Vec::new();
    let mut findings = Vec::new();

    match hardware {
        Ok(snapshot) => {
            append_hardware_diagnostics(&snapshot, &mut checks);
            let (rfkill_check, finding) = evaluate_radio_block(
                Subsystem::Bluetooth,
                "bluetooth",
                "bluetooth.radio.block_state",
                "BT-001",
                snapshot.rfkill_devices.as_deref(),
            );
            checks.push(rfkill_check);
            findings.extend(finding);
            if !snapshot.warnings.is_empty() {
                checks.push(DiagnosticCheck::new(
                    "bluetooth.hardware.completeness",
                    "Hardware inventory completeness",
                    DiagnosticStatus::Warning,
                    format!(
                        "{} Bluetooth hardware attribute(s) could not be read or interpreted.",
                        snapshot.warnings.len()
                    ),
                    snapshot
                        .warnings
                        .iter()
                        .map(|warning| Evidence::new("sysfs", warning.clone()))
                        .collect(),
                ));
            }
        }
        Err(error) => {
            checks.push(DiagnosticCheck::new(
                "bluetooth.hardware.detected",
                "Bluetooth adapter",
                DiagnosticStatus::Unknown,
                "PRYN could not inspect the kernel Bluetooth controller inventory.",
                vec![Evidence::new(
                    "sysfs:/sys/class/bluetooth",
                    error.to_string(),
                )],
            ));
            checks.push(DiagnosticCheck::new(
                "bluetooth.driver.association",
                "Device driver",
                DiagnosticStatus::Unknown,
                "Bluetooth driver associations could not be inspected because the hardware inventory is unavailable.",
                Vec::new(),
            ));
            checks.push(DiagnosticCheck::new(
                "bluetooth.radio.block_state",
                "Radio block state",
                DiagnosticStatus::Unknown,
                "Bluetooth rfkill state could not be inspected because the hardware inventory is unavailable.",
                Vec::new(),
            ));
        }
    }

    append_bluez_diagnostics(bluez, &mut checks, &mut findings);
    SubsystemReport::new(Subsystem::Bluetooth, checks, findings)
}

fn append_hardware_diagnostics(snapshot: &BluetoothSnapshot, checks: &mut Vec<DiagnosticCheck>) {
    let controller_check = if !snapshot.class_available {
        DiagnosticCheck::new(
            "bluetooth.hardware.detected",
            "Bluetooth adapter",
            DiagnosticStatus::Unknown,
            "The kernel Bluetooth class is not exposed. This does not prove the computer has no Bluetooth hardware.",
            vec![Evidence::new(
                "sysfs:/sys/class/bluetooth",
                "The Bluetooth class directory is unavailable.",
            )],
        )
    } else if snapshot.controllers.is_empty() {
        DiagnosticCheck::new(
            "bluetooth.hardware.detected",
            "Bluetooth adapter",
            DiagnosticStatus::Unknown,
            "No Bluetooth controller was discovered. A driver may be unavailable, the device may be disabled, or this system may not contain Bluetooth hardware.",
            vec![Evidence::new(
                "sysfs:/sys/class/bluetooth",
                "The class directory exists but contains no `hci*` controller entries.",
            )],
        )
    } else {
        let names = snapshot
            .controllers
            .iter()
            .map(|controller| controller.name.as_str())
            .collect::<Vec<_>>();
        DiagnosticCheck::new(
            "bluetooth.hardware.detected",
            "Bluetooth adapter",
            DiagnosticStatus::Pass,
            format!(
                "Detected {} Bluetooth controller(s): {}.",
                names.len(),
                names.join(", ")
            ),
            names
                .iter()
                .map(|name| {
                    Evidence::new(
                        "sysfs:/sys/class/bluetooth",
                        format!("Controller `{name}` was discovered."),
                    )
                })
                .collect(),
        )
    };
    checks.push(controller_check);

    let driver_count = snapshot
        .controllers
        .iter()
        .filter(|controller| controller.driver.is_some())
        .count();
    let driver_status =
        if !snapshot.controllers.is_empty() && driver_count == snapshot.controllers.len() {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Unknown
        };
    checks.push(DiagnosticCheck::new(
        "bluetooth.driver.association",
        "Device driver",
        driver_status,
        if driver_count == 0 {
            "No Bluetooth controller driver association was exposed.".to_owned()
        } else {
            format!(
                "Driver associations were exposed for {driver_count} of {} controller(s).",
                snapshot.controllers.len()
            )
        },
        snapshot
            .controllers
            .iter()
            .filter_map(|controller| {
                controller.driver.as_ref().map(|driver| {
                    Evidence::new(
                        format!(
                            "sysfs:/sys/class/bluetooth/{}/device/driver",
                            controller.name
                        ),
                        format!("Driver: {driver}"),
                    )
                })
            })
            .collect(),
    ));
}

fn append_bluez_diagnostics(
    probe: BluezProbe,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let snapshot = match probe {
        BluezProbe::Available(snapshot) => {
            checks.push(DiagnosticCheck::new(
                "bluetooth.bluez.service",
                "Bluetooth service",
                DiagnosticStatus::Pass,
                "BlueZ owns its system D-Bus service name and returned a managed-object snapshot.",
                vec![Evidence::new(
                    "system D-Bus",
                    "org.bluez was available and its ObjectManager.GetManagedObjects call succeeded.",
                )],
            ));
            snapshot
        }
        BluezProbe::NotAvailable => {
            checks.push(DiagnosticCheck::new(
                "bluetooth.bluez.service",
                "Bluetooth service",
                DiagnosticStatus::Unknown,
                "BlueZ does not currently own its system D-Bus service name. Bluetooth hardware may still be present, but adapter and device state cannot be confirmed.",
                vec![Evidence::new("system D-Bus", "No owner for org.bluez was reported.")],
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.inventory",
                "BlueZ adapters",
                "Adapter inventory is unavailable because the BlueZ service is not running or not registered.",
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.powered",
                "Adapter power state",
                "PRYN cannot confirm whether a Bluetooth adapter is powered on without BlueZ.",
            ));
            append_unavailable_device_checks(checks);
            return;
        }
        BluezProbe::SystemBusUnavailable => {
            checks.push(DiagnosticCheck::new(
                "bluetooth.bluez.service",
                "Bluetooth service",
                DiagnosticStatus::Unknown,
                "PRYN could not connect to the system D-Bus, so BlueZ service state is unknown.",
                Vec::new(),
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.inventory",
                "BlueZ adapters",
                "Adapter inventory could not be queried because the system D-Bus is unavailable.",
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.powered",
                "Adapter power state",
                "Adapter power state could not be queried because the system D-Bus is unavailable.",
            ));
            append_unavailable_device_checks(checks);
            return;
        }
        BluezProbe::QueryFailed => {
            checks.push(DiagnosticCheck::new(
                "bluetooth.bluez.service",
                "Bluetooth service",
                DiagnosticStatus::Unknown,
                "PRYN could not confirm the BlueZ service state or read its managed-object inventory.",
                Vec::new(),
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.inventory",
                "BlueZ adapters",
                "BlueZ adapter inventory could not be read.",
            ));
            checks.push(unknown_bluez_check(
                "bluetooth.adapter.powered",
                "Adapter power state",
                "Adapter power state could not be read.",
            ));
            append_unavailable_device_checks(checks);
            return;
        }
    };

    append_adapter_diagnostics(&snapshot.adapters, checks, findings);
    append_device_diagnostics(&snapshot.devices, checks);

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "bluetooth.bluez.completeness",
            "BlueZ inventory completeness",
            DiagnosticStatus::Warning,
            format!(
                "{} adapter attribute(s) could not be interpreted from BlueZ.",
                snapshot.warnings.len()
            ),
            snapshot
                .warnings
                .iter()
                .map(|warning| Evidence::new("BlueZ D-Bus", warning.clone()))
                .collect(),
        ));
    }
}

fn append_adapter_diagnostics(
    adapters: &[BluezAdapter],
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    if adapters.is_empty() {
        checks.push(unknown_bluez_check(
            "bluetooth.adapter.inventory",
            "BlueZ adapters",
            "BlueZ is running but did not expose an Adapter1 object. The controller may be unavailable or not initialized.",
        ));
        checks.push(unknown_bluez_check(
            "bluetooth.adapter.powered",
            "Adapter power state",
            "No BlueZ adapter was available for a power-state check.",
        ));
        checks.push(DiagnosticCheck::new(
            "bluetooth.adapter.visibility",
            "Adapter visibility",
            DiagnosticStatus::NotApplicable,
            "Adapter visibility and pairing state cannot be evaluated because BlueZ exposed no adapter.",
            Vec::new(),
        ));
        return;
    }

    let adapter_names = adapters
        .iter()
        .map(|adapter| adapter.name.as_str())
        .collect::<Vec<_>>();
    checks.push(DiagnosticCheck::new(
        "bluetooth.adapter.inventory",
        "BlueZ adapters",
        DiagnosticStatus::Pass,
        format!(
            "BlueZ exposed {} adapter(s): {}.",
            adapters.len(),
            adapter_names.join(", ")
        ),
        vec![Evidence::new(
            "BlueZ Adapter1",
            "Adapter inventory was read through ObjectManager without starting discovery or changing settings.",
        )],
    ));

    let powered_count = adapters
        .iter()
        .filter(|adapter| adapter.powered == Some(true))
        .count();
    let unpowered_count = adapters
        .iter()
        .filter(|adapter| adapter.powered == Some(false))
        .count();
    let unknown_count = adapters.len() - powered_count - unpowered_count;

    if powered_count > 0 {
        checks.push(DiagnosticCheck::new(
            "bluetooth.adapter.powered",
            "Adapter power state",
            DiagnosticStatus::Pass,
            format!(
                "{powered_count} of {} Bluetooth adapter(s) are powered on; {unpowered_count} are off and {unknown_count} have an unknown state.",
                adapters.len()
            ),
            adapter_evidence(adapters),
        ));
    } else if unpowered_count == adapters.len() {
        let summary = format!(
            "All {} BlueZ adapter(s) are powered off. Bluetooth cannot connect to devices while every adapter is off.",
            adapters.len()
        );
        checks.push(DiagnosticCheck::new(
            "bluetooth.adapter.powered",
            "Adapter power state",
            DiagnosticStatus::Warning,
            summary.clone(),
            adapter_evidence(adapters),
        ));
        findings.push(Finding::new(
            "BT-002",
            Severity::Low,
            "Bluetooth adapter is powered off",
            summary,
            adapter_evidence(adapters),
            vec![
                "Turn Bluetooth on in your desktop settings if you intend to use a Bluetooth device.".to_owned(),
                "Check airplane mode and the laptop's wireless hardware key if the adapter will not power on.".to_owned(),
                "Run `pryn check bluetooth` again after changing the setting.".to_owned(),
            ],
        ));
    } else {
        checks.push(DiagnosticCheck::new(
            "bluetooth.adapter.powered",
            "Adapter power state",
            DiagnosticStatus::Unknown,
            "BlueZ did not expose a confirmed powered-on adapter or a complete set of power states.",
            adapter_evidence(adapters),
        ));
    }

    let discovering = adapters
        .iter()
        .filter(|adapter| adapter.discovering == Some(true))
        .count();
    let discoverable = adapters
        .iter()
        .filter(|adapter| adapter.discoverable == Some(true))
        .count();
    let pairable = adapters
        .iter()
        .filter(|adapter| adapter.pairable == Some(true))
        .count();
    let visibility_unknown = adapters
        .iter()
        .filter(|adapter| {
            adapter.discovering.is_none()
                || adapter.discoverable.is_none()
                || adapter.pairable.is_none()
        })
        .count();
    checks.push(DiagnosticCheck::new(
        "bluetooth.adapter.visibility",
        "Adapter visibility and pairing",
        if visibility_unknown == 0 {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Unknown
        },
        format!(
            "{discoverable} adapter(s) are discoverable, {pairable} are pairable, and {discovering} are actively scanning; {visibility_unknown} adapter(s) have incomplete visibility state. These are reported states, not automatic faults."
        ),
        adapter_evidence(adapters),
    ));
}

fn append_device_diagnostics(
    devices: &[crate::platform::linux::bluez::BluezDevice],
    checks: &mut Vec<DiagnosticCheck>,
) {
    let paired = devices
        .iter()
        .filter(|device| device.paired == Some(true))
        .count();
    let connected = devices
        .iter()
        .filter(|device| device.connected == Some(true))
        .count();
    let trusted = devices
        .iter()
        .filter(|device| device.trusted == Some(true))
        .count();
    let blocked = devices
        .iter()
        .filter(|device| device.blocked == Some(true))
        .count();
    let unresolved = devices
        .iter()
        .filter(|device| device.connected == Some(true) && device.services_resolved == Some(false))
        .count();
    let incomplete = devices
        .iter()
        .filter(|device| {
            device.paired.is_none()
                || device.trusted.is_none()
                || device.connected.is_none()
                || device.blocked.is_none()
                || (device.connected == Some(true) && device.services_resolved.is_none())
        })
        .count();

    let device_status = if devices.is_empty() {
        DiagnosticStatus::NotApplicable
    } else if incomplete == 0 {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Unknown
    };
    checks.push(DiagnosticCheck::new(
        "bluetooth.devices.inventory",
        "Known Bluetooth devices",
        device_status,
        if devices.is_empty() {
            "BlueZ returned no known Bluetooth device objects. This is not an adapter or hardware failure.".to_owned()
        } else {
            format!(
                "BlueZ returned {} known device(s): {paired} paired, {connected} connected, {trusted} trusted, {blocked} blocked, {unresolved} connected service set(s) not yet resolved, and {incomplete} device(s) with incomplete state.",
                devices.len()
            )
        },
        devices
            .iter()
            .filter_map(|device| {
                let name = device.name.as_deref()?;
                Some(Evidence::new(
                    "BlueZ Device1",
                    format!(
                        "{name}: paired={:?}, connected={:?}, trusted={:?}, blocked={:?}, services_resolved={:?}.",
                        device.paired,
                        device.connected,
                        device.trusted,
                        device.blocked,
                        device.services_resolved
                    ),
                ))
            })
            .collect(),
    ));

    let connected_status = if connected > 0 {
        DiagnosticStatus::Pass
    } else if devices.iter().all(|device| device.connected.is_some()) {
        DiagnosticStatus::NotApplicable
    } else {
        DiagnosticStatus::Unknown
    };
    checks.push(DiagnosticCheck::new(
        "bluetooth.devices.connected",
        "Connected Bluetooth devices",
        connected_status,
        if connected > 0 {
            format!("BlueZ reports {connected} connected Bluetooth device(s).")
        } else if devices.is_empty() {
            "No Bluetooth device objects are currently exposed by BlueZ.".to_owned()
        } else {
            "BlueZ reports no connected devices. Pairing and connection are separate states; this is not treated as a fault.".to_owned()
        },
        Vec::new(),
    ));
}

fn append_unavailable_device_checks(checks: &mut Vec<DiagnosticCheck>) {
    checks.push(unknown_bluez_check(
        "bluetooth.devices.inventory",
        "Known Bluetooth devices",
        "Device inventory could not be queried from BlueZ.",
    ));
    checks.push(unknown_bluez_check(
        "bluetooth.devices.connected",
        "Connected Bluetooth devices",
        "Connection state could not be queried from BlueZ.",
    ));
}

fn unknown_bluez_check(
    id: &'static str,
    title: &'static str,
    summary: &'static str,
) -> DiagnosticCheck {
    DiagnosticCheck::new(id, title, DiagnosticStatus::Unknown, summary, Vec::new())
}

fn adapter_evidence(adapters: &[BluezAdapter]) -> Vec<Evidence> {
    adapters
        .iter()
        .map(|adapter| {
            Evidence::new(
                "BlueZ Adapter1",
                format!(
                    "{}: powered={:?}, discovering={:?}, discoverable={:?}, pairable={:?}.",
                    adapter.name,
                    adapter.powered,
                    adapter.discovering,
                    adapter.discoverable,
                    adapter.pairable
                ),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{append_adapter_diagnostics, append_bluez_diagnostics};
    use crate::model::DiagnosticStatus;
    use crate::platform::linux::bluez::{BluezAdapter, BluezDevice, BluezProbe, BluezSnapshot};

    fn adapter(powered: Option<bool>) -> BluezAdapter {
        BluezAdapter {
            name: "Laptop Bluetooth".to_owned(),
            powered,
            discovering: Some(false),
            discoverable: Some(false),
            pairable: Some(true),
        }
    }

    #[test]
    fn bluez_reports_powered_adapter_and_connected_devices() {
        let snapshot = BluezSnapshot {
            adapters: vec![adapter(Some(true))],
            devices: vec![BluezDevice {
                name: Some("Wireless Headphones".to_owned()),
                adapter: Some("hci0".to_owned()),
                paired: Some(true),
                trusted: Some(true),
                connected: Some(true),
                blocked: Some(false),
                services_resolved: Some(true),
            }],
            warnings: Vec::new(),
        };
        let mut checks = Vec::new();
        let mut findings = Vec::new();
        append_bluez_diagnostics(BluezProbe::Available(snapshot), &mut checks, &mut findings);

        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.bluez.service" && check.status == DiagnosticStatus::Pass
        }));
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.adapter.powered" && check.status == DiagnosticStatus::Pass
        }));
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.devices.connected" && check.status == DiagnosticStatus::Pass
        }));
        assert!(findings.is_empty());
    }

    #[test]
    fn all_adapters_powered_off_produces_a_specific_finding() {
        let mut checks = Vec::new();
        let mut findings = Vec::new();
        append_adapter_diagnostics(&[adapter(Some(false))], &mut checks, &mut findings);

        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.adapter.powered" && check.status == DiagnosticStatus::Warning
        }));
        assert!(findings.iter().any(|finding| finding.code == "BT-002"));
    }

    #[test]
    fn unavailable_bluez_is_unknown_not_unimplemented() {
        let mut checks = Vec::new();
        let mut findings = Vec::new();
        append_bluez_diagnostics(BluezProbe::NotAvailable, &mut checks, &mut findings);

        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.bluez.service" && check.status == DiagnosticStatus::Unknown
        }));
        assert!(
            checks
                .iter()
                .all(|check| check.status != DiagnosticStatus::NotImplemented)
        );
    }

    #[test]
    fn disconnected_paired_devices_are_not_reported_as_broken() {
        let devices = vec![BluezDevice {
            name: Some("Wireless Headphones".to_owned()),
            adapter: Some("hci0".to_owned()),
            paired: Some(true),
            trusted: Some(true),
            connected: Some(false),
            blocked: Some(false),
            services_resolved: Some(false),
        }];
        let mut checks = Vec::new();
        super::append_device_diagnostics(&devices, &mut checks);
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.devices.connected"
                && check.status == DiagnosticStatus::NotApplicable
        }));
    }

    #[test]
    fn empty_device_inventory_is_not_reported_as_a_successful_device_check() {
        let mut checks = Vec::new();
        super::append_device_diagnostics(&[], &mut checks);
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.devices.inventory"
                && check.status == DiagnosticStatus::NotApplicable
        }));
    }

    #[test]
    fn device_inventory_is_unknown_when_bluez_omits_device_state() {
        let devices = vec![BluezDevice {
            name: Some("Wireless Headphones".to_owned()),
            adapter: Some("hci0".to_owned()),
            paired: Some(true),
            trusted: None,
            connected: Some(false),
            blocked: Some(false),
            services_resolved: None,
        }];
        let mut checks = Vec::new();
        super::append_device_diagnostics(&devices, &mut checks);
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.devices.inventory"
                && check.status == DiagnosticStatus::Unknown
                && check.summary.contains("incomplete state")
        }));
    }

    #[test]
    fn adapter_visibility_is_unknown_when_bluez_omits_required_properties() {
        let mut incomplete_adapter = adapter(Some(true));
        incomplete_adapter.discovering = None;
        let mut checks = Vec::new();
        let mut findings = Vec::new();
        append_adapter_diagnostics(&[incomplete_adapter], &mut checks, &mut findings);
        assert!(checks.iter().any(|check| {
            check.id == "bluetooth.adapter.visibility" && check.status == DiagnosticStatus::Unknown
        }));
    }
}
