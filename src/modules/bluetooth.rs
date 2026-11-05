use crate::model::{DiagnosticCheck, DiagnosticStatus, Evidence, Subsystem, SubsystemReport};
use crate::platform::linux::{BluetoothSnapshot, LinuxCollector};

use super::rfkill::evaluate_radio_block;

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    match collector.collect_bluetooth() {
        Ok(snapshot) => evaluate_snapshot(snapshot),
        Err(error) => SubsystemReport::new(
            Subsystem::Bluetooth,
            vec![DiagnosticCheck::new(
                "bluetooth.inventory.available",
                "Bluetooth controller inventory",
                DiagnosticStatus::Unknown,
                "PRYN could not inspect the Bluetooth sysfs class.",
                vec![Evidence::new(
                    "sysfs:/sys/class/bluetooth",
                    error.to_string(),
                )],
            )],
            Vec::new(),
        ),
    }
}

fn evaluate_snapshot(snapshot: BluetoothSnapshot) -> SubsystemReport {
    let mut checks = Vec::new();

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

    let (rfkill_check, finding) = evaluate_radio_block(
        Subsystem::Bluetooth,
        "bluetooth",
        "bluetooth.radio.block_state",
        "BT-001",
        snapshot.rfkill_devices.as_deref(),
    );
    checks.push(rfkill_check);

    checks.push(DiagnosticCheck::new(
        "bluetooth.bluez.service",
        "Bluetooth service",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet confirm whether the Bluetooth service is running.",
        Vec::new(),
    ));
    checks.push(DiagnosticCheck::new(
        "bluetooth.connection.state",
        "Connected Bluetooth devices",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet check paired devices or active Bluetooth connections.",
        Vec::new(),
    ));

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "bluetooth.collection.completeness",
            "Collection completeness",
            DiagnosticStatus::Warning,
            format!(
                "{} system attribute(s) could not be read or interpreted.",
                snapshot.warnings.len()
            ),
            snapshot
                .warnings
                .iter()
                .map(|warning| Evidence::new("sysfs", warning.clone()))
                .collect(),
        ));
    }

    SubsystemReport::new(Subsystem::Bluetooth, checks, finding.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::evaluate_snapshot;
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::{BluetoothController, BluetoothSnapshot};

    #[test]
    fn controller_discovery_is_reported_without_claiming_bluez_is_running() {
        let report = evaluate_snapshot(BluetoothSnapshot {
            class_available: true,
            controllers: vec![BluetoothController {
                name: "hci0".to_owned(),
                driver: Some("btusb".to_owned()),
            }],
            rfkill_devices: Some(Vec::new()),
            warnings: Vec::new(),
        });

        assert_eq!(report.subsystem, Subsystem::Bluetooth);
        assert!(report.checks.iter().any(|check| {
            check.id == "bluetooth.hardware.detected" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "bluetooth.bluez.service"
                && check.status == DiagnosticStatus::NotImplemented
        }));
    }
}
