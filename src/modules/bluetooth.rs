use crate::{
    model::{CheckResult, CheckStatus, Evidence, Severity, Subsystem, SubsystemReport},
    modules::DiagnosticModule,
    platform::linux::LinuxPlatform,
};

/// Read-only Bluetooth diagnostics backed by Linux sysfs and rfkill.
pub struct BluetoothModule;

impl DiagnosticModule for BluetoothModule {
    fn subsystem(&self) -> Subsystem {
        Subsystem::Bluetooth
    }

    fn diagnose(&self, platform: &LinuxPlatform) -> SubsystemReport {
        let checks = vec![
            check_controllers(platform),
            check_radio_state(platform),
            check_bluez_state(),
        ];

        SubsystemReport::new(Subsystem::Bluetooth, checks)
    }
}

fn check_controllers(platform: &LinuxPlatform) -> CheckResult {
    match platform.bluetooth_controllers() {
        Ok(controllers) if !controllers.is_empty() => CheckResult::new(
            "BLUETOOTH.HARDWARE",
            "Kernel Bluetooth controllers",
            CheckStatus::Pass,
            Severity::Info,
            format!(
                "Found {count} Bluetooth controller(s).",
                count = controllers.len()
            ),
        )
        .with_evidence(Evidence::new("/sys/class/bluetooth", controllers.join(", "))),
        Ok(_) => CheckResult::new(
            "BLUETOOTH.HARDWARE",
            "Kernel Bluetooth controllers",
            CheckStatus::Problem,
            Severity::Medium,
            "No Bluetooth controller was exposed through /sys/class/bluetooth.",
        )
        .with_recommendation(
            "Check whether a Bluetooth adapter is present and whether its kernel driver is loaded. This result does not prove that physical hardware is absent.",
        ),
        Err(error) => CheckResult::new(
            "BLUETOOTH.HARDWARE",
            "Kernel Bluetooth controllers",
            CheckStatus::Unknown,
            Severity::Info,
            "PRYN could not inspect the kernel Bluetooth controller inventory.",
        )
        .with_evidence(Evidence::new("/sys/class/bluetooth", error.to_string())),
    }
}

fn check_radio_state(platform: &LinuxPlatform) -> CheckResult {
    match platform.rfkill_states("bluetooth") {
        Ok(states) if states.is_empty() => CheckResult::new(
            "BLUETOOTH.RADIO",
            "Bluetooth radio block state",
            CheckStatus::Unknown,
            Severity::Info,
            "No Bluetooth rfkill entries were found; the radio block state could not be established.",
        ),
        Ok(states) => {
            let blocked: Vec<_> = states
                .iter()
                .filter(|state| state.soft_blocked || state.hard_blocked)
                .collect();

            if blocked.is_empty() {
                CheckResult::new(
                    "BLUETOOTH.RADIO",
                    "Bluetooth radio block state",
                    CheckStatus::Pass,
                    Severity::Info,
                    "All discovered Bluetooth rfkill entries are unblocked.",
                )
                .with_evidence(Evidence::new(
                    "/sys/class/rfkill",
                    states
                        .iter()
                        .map(|state| state.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ))
            } else {
                let details = blocked
                    .iter()
                    .map(|state| {
                        format!(
                            "{name} (soft_blocked={soft_blocked}, hard_blocked={hard_blocked})",
                            name = state.name,
                            soft_blocked = state.soft_blocked,
                            hard_blocked = state.hard_blocked
                        )
                    })
                    .collect::<Vec<_>>();
                CheckResult::new(
                    "BLUETOOTH.RADIO",
                    "Bluetooth radio block state",
                    CheckStatus::Warning,
                    Severity::Medium,
                    "At least one Bluetooth radio is blocked by rfkill.",
                )
                .with_evidence(Evidence::new("/sys/class/rfkill", details.join("; ")))
                .with_recommendation(
                    "Check the hardware wireless switch and Bluetooth or airplane-mode controls. PRYN does not change radio state automatically.",
                )
            }
        }
        Err(error) => CheckResult::new(
            "BLUETOOTH.RADIO",
            "Bluetooth radio block state",
            CheckStatus::Unknown,
            Severity::Info,
            "PRYN could not read Bluetooth rfkill state.",
        )
        .with_evidence(Evidence::new("/sys/class/rfkill", error.to_string())),
    }
}

fn check_bluez_state() -> CheckResult {
    CheckResult::new(
        "BLUETOOTH.BLUEZ",
        "BlueZ service",
        CheckStatus::Unknown,
        Severity::Info,
        "BlueZ service and adapter state are not queried in this initial release.",
    )
    .with_recommendation(
        "A later provider will query BlueZ over D-Bus to inspect adapter power, discovery, pairing, and connection state.",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use crate::{
        model::CheckStatus,
        modules::{BluetoothModule, DiagnosticModule},
        platform::linux::{LinuxPaths, LinuxPlatform},
    };

    #[test]
    fn detects_kernel_bluetooth_controller() {
        let root = tempdir().expect("temporary directory should be created");
        fs::create_dir_all(root.path().join("sys/class/bluetooth/hci0"))
            .expect("Bluetooth fixture should be created");
        let platform = LinuxPlatform::new(LinuxPaths {
            etc_root: root.path().join("etc"),
            proc_root: root.path().join("proc"),
            sys_root: root.path().join("sys"),
        });

        let report = BluetoothModule.diagnose(&platform);
        assert_eq!(report.checks[0].status, CheckStatus::Pass);
    }

    #[test]
    fn reports_hard_blocked_bluetooth_radio_as_warning() {
        let root = tempdir().expect("temporary directory should be created");
        let rfkill = root.path().join("sys/class/rfkill/rfkill1");
        fs::create_dir_all(&rfkill).expect("rfkill fixture should be created");
        fs::write(rfkill.join("type"), "bluetooth\n").expect("type should be written");
        fs::write(rfkill.join("soft"), "0\n").expect("soft state should be written");
        fs::write(rfkill.join("hard"), "1\n").expect("hard state should be written");
        let platform = LinuxPlatform::new(LinuxPaths {
            etc_root: root.path().join("etc"),
            proc_root: root.path().join("proc"),
            sys_root: root.path().join("sys"),
        });

        let report = BluetoothModule.diagnose(&platform);
        assert_eq!(report.checks[1].status, CheckStatus::Warning);
    }

    #[test]
    fn missing_sysfs_directory_is_unknown_not_a_hardware_failure() {
        let root = tempdir().expect("temporary directory should be created");
        let platform = LinuxPlatform::new(LinuxPaths {
            etc_root: root.path().join("etc"),
            proc_root: root.path().join("proc"),
            sys_root: root.path().join("sys"),
        });

        let report = BluetoothModule.diagnose(&platform);
        assert_eq!(report.checks[0].status, CheckStatus::Unknown);
    }
}
