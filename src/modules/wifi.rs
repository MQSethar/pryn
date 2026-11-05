use crate::model::{
    DiagnosticCheck, DiagnosticStatus, Evidence, Finding, Severity, Subsystem, SubsystemReport,
};
use crate::platform::linux::{LinuxCollector, ProbeError, WifiSnapshot};

use super::rfkill::evaluate_radio_block;

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    match collector.collect_wifi() {
        Ok(snapshot) => evaluate_snapshot(snapshot),
        Err(error) => report_collection_error(error),
    }
}

fn report_collection_error(error: ProbeError) -> SubsystemReport {
    SubsystemReport::new(
        Subsystem::Wifi,
        vec![DiagnosticCheck::new(
            "wifi.inventory.available",
            "Wireless interface inventory",
            DiagnosticStatus::Unknown,
            "PRYN could not read the kernel network-interface inventory.",
            vec![Evidence::new("sysfs:/sys/class/net", error.to_string())],
        )],
        Vec::new(),
    )
}

fn evaluate_snapshot(snapshot: WifiSnapshot) -> SubsystemReport {
    let mut checks = Vec::new();
    let mut findings = Vec::new();

    if snapshot.interfaces.is_empty() {
        checks.push(DiagnosticCheck::new(
            "wifi.hardware.detected",
            "Wi-Fi adapter",
            DiagnosticStatus::Unknown,
            "No wireless network interface was discovered in sysfs. This does not prove that the computer has no Wi-Fi hardware; the driver may be unavailable or the device may not be exposed to this environment.",
            vec![Evidence::new(
                "sysfs:/sys/class/net",
                "No interface exposing a `wireless` directory or `phy80211` link was found.",
            )],
        ));
    } else {
        let names = snapshot
            .interfaces
            .iter()
            .map(|interface| interface.name.as_str())
            .collect::<Vec<_>>();
        checks.push(DiagnosticCheck::new(
            "wifi.hardware.detected",
            "Wi-Fi adapter",
            DiagnosticStatus::Pass,
            format!(
                "Detected {} wireless interface(s): {}.",
                names.len(),
                names.join(", ")
            ),
            names
                .iter()
                .map(|name| {
                    Evidence::new(
                        "sysfs:/sys/class/net",
                        format!("Wireless interface `{name}` was discovered."),
                    )
                })
                .collect(),
        ));

        let driver_details = snapshot
            .interfaces
            .iter()
            .filter_map(|interface| {
                interface
                    .driver
                    .as_ref()
                    .map(|driver| format!("{}: {driver}", interface.name))
            })
            .collect::<Vec<_>>();
        let all_have_drivers = snapshot
            .interfaces
            .iter()
            .all(|interface| interface.driver.is_some());
        let driver_status = if all_have_drivers {
            DiagnosticStatus::Pass
        } else {
            DiagnosticStatus::Unknown
        };
        let driver_summary = if driver_details.is_empty() {
            "No driver association was exposed through the interface device links.".to_owned()
        } else {
            format!(
                "Driver associations observed: {}.",
                driver_details.join("; ")
            )
        };
        checks.push(DiagnosticCheck::new(
            "wifi.driver.association",
            "Device driver",
            driver_status,
            driver_summary,
            driver_details
                .iter()
                .map(|detail| Evidence::new("sysfs:/sys/class/net/*/device/driver", detail.clone()))
                .collect(),
        ));

        let up_interfaces = snapshot
            .interfaces
            .iter()
            .filter(|interface| interface.operstate.as_deref() == Some("up"))
            .map(|interface| interface.name.as_str())
            .collect::<Vec<_>>();
        let down_interfaces = snapshot
            .interfaces
            .iter()
            .filter(|interface| interface.operstate.as_deref() == Some("down"))
            .map(|interface| interface.name.as_str())
            .collect::<Vec<_>>();
        let (operstate, summary) = if !up_interfaces.is_empty() {
            (
                DiagnosticStatus::Pass,
                format!(
                    "Interfaces reporting operational state `up`: {}.",
                    up_interfaces.join(", ")
                ),
            )
        } else if !down_interfaces.is_empty() {
            (
                DiagnosticStatus::Warning,
                format!(
                    "Interfaces reporting operational state `down`: {}.",
                    down_interfaces.join(", ")
                ),
            )
        } else {
            (
                DiagnosticStatus::Unknown,
                "No wireless interface reported a conclusive operational state.".to_owned(),
            )
        };
        checks.push(DiagnosticCheck::new(
            "wifi.interface.operstate",
            "Interface operational state",
            operstate,
            summary,
            snapshot
                .interfaces
                .iter()
                .filter_map(|interface| {
                    interface.operstate.as_ref().map(|state| {
                        Evidence::new(
                            format!("sysfs:/sys/class/net/{}/operstate", interface.name),
                            format!("Operational state: {state}"),
                        )
                    })
                })
                .collect(),
        ));

        let carrier_up = snapshot
            .interfaces
            .iter()
            .any(|interface| interface.carrier == Some(true));
        let all_carrier_down = snapshot
            .interfaces
            .iter()
            .all(|interface| interface.carrier == Some(false));
        let (carrier_status, carrier_summary) = if carrier_up {
            (
                DiagnosticStatus::Pass,
                "At least one wireless interface reports a lower-layer carrier.".to_owned(),
            )
        } else if all_carrier_down {
            (
                DiagnosticStatus::Warning,
                "All discovered wireless interfaces report no carrier. This indicates that a link is not currently established; it does not identify the cause or prove internet connectivity is unavailable.".to_owned(),
            )
        } else {
            (
                DiagnosticStatus::Unknown,
                "Carrier state is unavailable or incomplete for the discovered wireless interfaces.".to_owned(),
            )
        };
        checks.push(DiagnosticCheck::new(
            "wifi.interface.carrier",
            "Wireless link carrier",
            carrier_status,
            carrier_summary.clone(),
            snapshot
                .interfaces
                .iter()
                .filter_map(|interface| {
                    interface.carrier.map(|carrier| {
                        Evidence::new(
                            format!("sysfs:/sys/class/net/{}/carrier", interface.name),
                            format!("Carrier: {}", if carrier { "present" } else { "absent" }),
                        )
                    })
                })
                .collect(),
        ));

        if all_carrier_down {
            findings.push(Finding::new(
                "WIFI-002",
                Severity::Low,
                "No wireless link carrier",
                carrier_summary,
                vec![Evidence::new(
                    "sysfs:/sys/class/net/*/carrier",
                    "Every discovered wireless interface reported carrier value `0`.",
                )],
                vec![
                    "If you expect an active connection, connect to a wireless network using your desktop's network controls.".to_owned(),
                    "Rerun PRYN after connecting. A carrier state alone does not verify IP, routing, DNS, or internet access.".to_owned(),
                ],
            ));
        }
    }

    let (rfkill_check, rfkill_finding) = evaluate_radio_block(
        Subsystem::Wifi,
        "wlan",
        "wifi.radio.block_state",
        "WIFI-001",
        snapshot.rfkill_devices.as_deref(),
    );
    checks.push(rfkill_check);
    if let Some(finding) = rfkill_finding {
        findings.push(finding);
    }

    checks.push(DiagnosticCheck::new(
        "wifi.network_manager.state",
        "Wi-Fi connection service",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet confirm which service manages Wi-Fi or whether you are connected to a network.",
        Vec::new(),
    ));
    checks.push(DiagnosticCheck::new(
        "wifi.ip_configuration",
        "Network settings",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet check the network address, route, or DNS settings.",
        Vec::new(),
    ));

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "wifi.collection.completeness",
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

    SubsystemReport::new(Subsystem::Wifi, checks, findings)
}

#[cfg(test)]
mod tests {
    use super::evaluate_snapshot;
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::{RfkillDevice, WifiInterface, WifiSnapshot};

    fn sample_snapshot(carrier: Option<bool>, blocked: bool) -> WifiSnapshot {
        WifiSnapshot {
            interfaces: vec![WifiInterface {
                name: "wlp2s0".to_owned(),
                operstate: Some("up".to_owned()),
                carrier,
                driver: Some("iwlwifi".to_owned()),
            }],
            rfkill_devices: Some(vec![RfkillDevice {
                name: Some("phy0".to_owned()),
                kind: "wlan".to_owned(),
                soft_blocked: Some(blocked),
                hard_blocked: Some(false),
            }]),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn wireless_link_does_not_claim_internet_connectivity() {
        let report = evaluate_snapshot(sample_snapshot(Some(true), false));
        assert_eq!(report.subsystem, Subsystem::Wifi);
        assert!(report.checks.iter().any(|check| {
            check.id == "wifi.interface.carrier" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "wifi.ip_configuration" && check.status == DiagnosticStatus::NotImplemented
        }));
    }

    #[test]
    fn blocked_radio_produces_a_problem_finding() {
        let report = evaluate_snapshot(sample_snapshot(Some(false), true));
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "WIFI-001")
        );
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.id == "wifi.radio.block_state")
                .map(|check| check.status),
            Some(DiagnosticStatus::Problem)
        );
    }

    #[test]
    fn no_wireless_interfaces_remains_unknown() {
        let report = evaluate_snapshot(WifiSnapshot {
            interfaces: Vec::new(),
            rfkill_devices: None,
            warnings: Vec::new(),
        });
        assert_eq!(
            report
                .checks
                .iter()
                .find(|check| check.id == "wifi.hardware.detected")
                .map(|check| check.status),
            Some(DiagnosticStatus::Unknown)
        );
    }
}
