use crate::{
    model::{CheckResult, CheckStatus, Evidence, Severity, Subsystem, SubsystemReport},
    modules::DiagnosticModule,
    platform::linux::LinuxPlatform,
};

/// Read-only Wi-Fi diagnostics backed by Linux sysfs and rfkill.
pub struct WifiModule;

impl DiagnosticModule for WifiModule {
    fn subsystem(&self) -> Subsystem {
        Subsystem::Wifi
    }

    fn diagnose(&self, platform: &LinuxPlatform) -> SubsystemReport {
        let checks = vec![
            check_wireless_interfaces(platform),
            check_radio_state(platform),
            check_connection_state(),
        ];

        SubsystemReport::new(Subsystem::Wifi, checks)
    }
}

fn check_wireless_interfaces(platform: &LinuxPlatform) -> CheckResult {
    match platform.wifi_devices() {
        Ok(devices) if !devices.is_empty() => {
            let mut result = CheckResult::new(
                "WIFI.HARDWARE",
                "Kernel wireless interfaces",
                CheckStatus::Pass,
                Severity::Info,
                format!(
                    "Found {count} kernel wireless interface or PHY entries.",
                    count = devices.len()
                ),
            );

            for device in devices {
                result.evidence.push(Evidence::new("sysfs", device));
            }

            result
        }
        Ok(_) => CheckResult::new(
            "WIFI.HARDWARE",
            "Kernel wireless interfaces",
            CheckStatus::Problem,
            Severity::Medium,
            "No wireless interface or PHY was exposed through the inspected kernel interfaces.",
        )
        .with_evidence(Evidence::new(
            "/sys/class/net and /sys/class/ieee80211",
            "Both locations were inspected; no wireless interface or PHY entry was found.",
        ))
        .with_recommendation(
            "Check that a wireless adapter is present and that its kernel driver is loaded. A device that lacks a working driver may not appear here.",
        ),
        Err(error) => CheckResult::new(
            "WIFI.HARDWARE",
            "Kernel wireless interfaces",
            CheckStatus::Unknown,
            Severity::Info,
            "PRYN could not inspect the kernel wireless interface inventory.",
        )
        .with_evidence(Evidence::new("sysfs", error.to_string()))
        .with_recommendation(
            "Check that sysfs is mounted and readable. Run PRYN as your normal user first; do not use root merely to hide an unexpected permission problem.",
        ),
    }
}

fn check_radio_state(platform: &LinuxPlatform) -> CheckResult {
    match platform.rfkill_states("wlan") {
        Ok(states) if states.is_empty() => CheckResult::new(
            "WIFI.RADIO",
            "Wi-Fi radio block state",
            CheckStatus::Unknown,
            Severity::Info,
            "No Wi-Fi rfkill entries were found; the radio block state could not be established.",
        )
        .with_evidence(Evidence::new(
            "/sys/class/rfkill",
            "No entries with type 'wlan' were found.",
        )),
        Ok(states) => {
            let blocked: Vec<_> = states
                .iter()
                .filter(|state| state.soft_blocked || state.hard_blocked)
                .collect();

            if blocked.is_empty() {
                let names = states
                    .iter()
                    .map(|state| state.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                CheckResult::new(
                    "WIFI.RADIO",
                    "Wi-Fi radio block state",
                    CheckStatus::Pass,
                    Severity::Info,
                    "All discovered Wi-Fi rfkill entries are unblocked.",
                )
                .with_evidence(Evidence::new("/sys/class/rfkill", names))
            } else {
                let descriptions = blocked
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
                    "WIFI.RADIO",
                    "Wi-Fi radio block state",
                    CheckStatus::Warning,
                    Severity::Medium,
                    "At least one Wi-Fi radio is blocked by rfkill.",
                )
                .with_evidence(Evidence::new("/sys/class/rfkill", descriptions.join("; ")))
                .with_recommendation(
                    "Check the hardware wireless switch and software airplane-mode controls. Do not change the block state without the user's consent.",
                )
            }
        }
        Err(error) => CheckResult::new(
            "WIFI.RADIO",
            "Wi-Fi radio block state",
            CheckStatus::Unknown,
            Severity::Info,
            "PRYN could not read Wi-Fi rfkill state.",
        )
        .with_evidence(Evidence::new("/sys/class/rfkill", error.to_string())),
    }
}

fn check_connection_state() -> CheckResult {
    CheckResult::new(
        "WIFI.CONNECTION",
        "Network connection",
        CheckStatus::Unknown,
        Severity::Info,
        "Connection-manager state is not inspected in this initial release.",
    )
    .with_recommendation(
        "NetworkManager and iwd provider integrations will add connection, address, route, and DNS checks in a later release.",
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use crate::{
        model::{CheckStatus, Subsystem},
        modules::{DiagnosticModule, WifiModule},
        platform::linux::{LinuxPaths, LinuxPlatform},
    };

    fn platform(root: &Path) -> LinuxPlatform {
        LinuxPlatform::new(LinuxPaths {
            etc_root: root.join("etc"),
            proc_root: root.join("proc"),
            sys_root: root.join("sys"),
        })
    }

    #[test]
    fn detects_wireless_interface_from_sysfs_fixture() {
        let root = tempdir().expect("temporary directory should be created");
        let wireless = root.path().join("sys/class/net/wlp1s0/wireless");
        fs::create_dir_all(&wireless).expect("wireless fixture should be created");

        let report = WifiModule.diagnose(&platform(root.path()));
        assert_eq!(report.subsystem, Subsystem::Wifi);
        assert_eq!(report.checks[0].status, CheckStatus::Pass);
        assert!(report.checks[0].evidence[0].detail.contains("wlp1s0"));
    }

    #[test]
    fn reports_soft_blocked_wifi_radio_as_warning() {
        let root = tempdir().expect("temporary directory should be created");
        let rfkill = root.path().join("sys/class/rfkill/rfkill0");
        fs::create_dir_all(&rfkill).expect("rfkill fixture should be created");
        fs::write(rfkill.join("type"), "wlan\n").expect("type should be written");
        fs::write(rfkill.join("soft"), "1\n").expect("soft state should be written");
        fs::write(rfkill.join("hard"), "0\n").expect("hard state should be written");

        let report = WifiModule.diagnose(&platform(root.path()));
        assert_eq!(report.checks[1].status, CheckStatus::Warning);
    }

    #[test]
    fn reports_missing_wireless_interface_as_problem_only_when_sysfs_is_readable() {
        let root = tempdir().expect("temporary directory should be created");
        fs::create_dir_all(root.path().join("sys/class/net"))
            .expect("network sysfs fixture should be created");
        fs::create_dir_all(root.path().join("sys/class/ieee80211"))
            .expect("wireless PHY fixture should be created");

        let report = WifiModule.diagnose(&platform(root.path()));
        assert_eq!(report.checks[0].status, CheckStatus::Problem);
    }
}
