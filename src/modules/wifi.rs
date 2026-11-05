use crate::model::{
    DiagnosticCheck, DiagnosticStatus, Evidence, Finding, Severity, Subsystem,
    SubsystemReport,
};
use crate::platform::linux::network_manager::{
    NetworkManagerConnectivity, NetworkManagerDeviceState, NetworkManagerSnapshot,
    NetworkManagerWifiDevice, WifiManagerProbe,
};
use crate::platform::linux::{LinuxCollector, ProbeError, RfkillDevice, WifiSnapshot};

use super::rfkill::evaluate_radio_block;

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    // The system-bus query is read-only and bounded by a short method timeout.
    // Collect it independently so a sysfs permission issue does not hide the
    // connection manager's view of the device.
    let manager_probe =
        crate::platform::linux::network_manager::collect_wifi_state();

    match collector.collect_wifi() {
        Ok(snapshot) => evaluate_snapshot(snapshot, manager_probe),
        Err(error) => report_collection_error(error, manager_probe),
    }
}

fn report_collection_error(
    error: ProbeError,
    manager_probe: WifiManagerProbe,
) -> SubsystemReport {
    let mut checks = vec![DiagnosticCheck::new(
        "wifi.inventory.available",
        "Wi-Fi adapter inventory",
        DiagnosticStatus::Unknown,
        "PRYN could not read the kernel network-interface inventory.",
        vec![Evidence::new("Linux system files", error.to_string())],
    )];
    let (manager_checks, findings) =
        evaluate_network_manager(manager_probe, 0, WifiRfkillState::default());
    checks.extend(manager_checks);
    checks.push(ip_configuration_not_implemented());

    SubsystemReport::new(Subsystem::Wifi, checks, findings)
}

fn evaluate_snapshot(
    snapshot: WifiSnapshot,
    manager_probe: WifiManagerProbe,
) -> SubsystemReport {
    let mut checks = Vec::new();
    let mut findings = Vec::new();

    if snapshot.interfaces.is_empty() {
        checks.push(DiagnosticCheck::new(
            "wifi.hardware.detected",
            "Wi-Fi adapter",
            DiagnosticStatus::Unknown,
            "No wireless network interface was discovered in the kernel. This does not prove that the computer has no Wi-Fi hardware; the driver may be unavailable or the device may not be exposed to this environment.",
            vec![Evidence::new(
                "Linux network interfaces",
                "No interface exposing a wireless sysfs entry was found.",
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
                        "Linux network interfaces",
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
            "No driver association was exposed for the wireless interfaces.".to_owned()
        } else {
            format!("Driver associations observed: {}.", driver_details.join("; "))
        };
        checks.push(DiagnosticCheck::new(
            "wifi.driver.association",
            "Device driver",
            driver_status,
            driver_summary,
            driver_details
                .iter()
                .map(|detail| Evidence::new("Linux device information", detail.clone()))
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
                            "Linux network interface state",
                            format!("{}: operational state is {state}.", interface.name),
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
                "All discovered wireless interfaces report no carrier. This means no wireless link is currently established; it does not identify the cause or prove internet connectivity is unavailable.".to_owned(),
            )
        } else {
            (
                DiagnosticStatus::Unknown,
                "Carrier state is unavailable or incomplete for the discovered wireless interfaces.".to_owned(),
            )
        };
        checks.push(DiagnosticCheck::new(
            "wifi.interface.carrier",
            "Wireless link",
            carrier_status,
            carrier_summary.clone(),
            snapshot
                .interfaces
                .iter()
                .filter_map(|interface| {
                    interface.carrier.map(|carrier| {
                        Evidence::new(
                            "Linux network interface state",
                            format!(
                                "{}: carrier is {}.",
                                interface.name,
                                if carrier { "present" } else { "absent" }
                            ),
                        )
                    })
                })
                .collect(),
        ));

        // A missing carrier is not itself a confirmed fault: a user may simply
        // be disconnected. NetworkManager's connection state below provides
        // the more useful context, so this remains a warning without a finding.
    }

    let rfkill_state = summarize_wlan_rfkill(snapshot.rfkill_devices.as_deref());
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

    let (manager_checks, manager_findings) = evaluate_network_manager(
        manager_probe,
        snapshot.interfaces.len(),
        rfkill_state,
    );
    checks.extend(manager_checks);
    findings.extend(manager_findings);

    checks.push(ip_configuration_not_implemented());

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
                .map(|warning| Evidence::new("Linux system information", warning.clone()))
                .collect(),
        ));
    }

    SubsystemReport::new(Subsystem::Wifi, checks, findings)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WifiRfkillState {
    soft_blocked: Option<bool>,
    hard_blocked: Option<bool>,
}

fn summarize_wlan_rfkill(devices: Option<&[RfkillDevice]>) -> WifiRfkillState {
    let Some(devices) = devices else {
        return WifiRfkillState::default();
    };

    let matching: Vec<&RfkillDevice> = devices
        .iter()
        .filter(|device| device.kind == "wlan")
        .collect();

    fn aggregate(values: impl Iterator<Item = Option<bool>>) -> Option<bool> {
        let values: Vec<Option<bool>> = values.collect();
        if values.is_empty() {
            None
        } else if values.contains(&Some(true)) {
            Some(true)
        } else if values.iter().all(|value| *value == Some(false)) {
            Some(false)
        } else {
            None
        }
    }

    WifiRfkillState {
        soft_blocked: aggregate(matching.iter().map(|device| device.soft_blocked)),
        hard_blocked: aggregate(matching.iter().map(|device| device.hard_blocked)),
    }
}

fn evaluate_network_manager(
    probe: WifiManagerProbe,
    kernel_wifi_interface_count: usize,
    rfkill_state: WifiRfkillState,
) -> (Vec<DiagnosticCheck>, Vec<Finding>) {
    let mut checks = Vec::new();
    let mut findings = Vec::new();

    match probe {
        WifiManagerProbe::NetworkManager(snapshot) => {
            checks.push(DiagnosticCheck::new(
                "wifi.connection_service.available",
                "Wi-Fi connection service",
                DiagnosticStatus::Pass,
                "NetworkManager is available and can be queried.",
                vec![Evidence::new(
                    "NetworkManager",
                    "The NetworkManager system service owns its D-Bus name.",
                )],
            ));

            evaluate_networking_enabled(&snapshot, &mut checks, &mut findings);
            evaluate_wireless_enabled(
                &snapshot,
                kernel_wifi_interface_count,
                rfkill_state,
                &mut checks,
                &mut findings,
            );
            evaluate_networkmanager_devices(
                &snapshot,
                kernel_wifi_interface_count,
                rfkill_state,
                &mut checks,
                &mut findings,
            );
            evaluate_connectivity(&snapshot, &mut checks, &mut findings);

            if !snapshot.warnings.is_empty() {
                checks.push(DiagnosticCheck::new(
                    "wifi.connection_service.completeness",
                    "Connection service information",
                    DiagnosticStatus::Warning,
                    format!(
                        "Some NetworkManager information could not be read ({} item(s)).",
                        snapshot.warnings.len()
                    ),
                    snapshot
                        .warnings
                        .iter()
                        .map(|warning| Evidence::new("NetworkManager", warning.clone()))
                        .collect(),
                ));
            }
        }
        WifiManagerProbe::IwdDetected => {
            checks.push(DiagnosticCheck::new(
                "wifi.connection_service.available",
                "Wi-Fi connection service",
                DiagnosticStatus::NotImplemented,
                "iwd is running. Direct iwd connection-state support is not available in this PRYN version yet.",
                vec![Evidence::new(
                    "system D-Bus",
                    "The iwd service owns the `net.connman.iwd` bus name.",
                )],
            ));
            checks.push(DiagnosticCheck::new(
                "wifi.connection.state",
                "Wi-Fi connection",
                DiagnosticStatus::NotImplemented,
                "PRYN has not yet read connection state directly from iwd.",
                Vec::new(),
            ));
        }
        WifiManagerProbe::NoKnownManager => {
            checks.push(DiagnosticCheck::new(
                "wifi.connection_service.available",
                "Wi-Fi connection service",
                DiagnosticStatus::Unknown,
                "NetworkManager and iwd were not found on the system bus. Another service or a manual network configuration may still manage Wi-Fi.",
                Vec::new(),
            ));
            checks.push(DiagnosticCheck::new(
                "wifi.connection.state",
                "Wi-Fi connection",
                DiagnosticStatus::Unknown,
                "PRYN could not confirm the Wi-Fi connection state because no supported connection service was found.",
                Vec::new(),
            ));
        }
        WifiManagerProbe::SystemBusUnavailable => {
            checks.push(DiagnosticCheck::new(
                "wifi.connection_service.available",
                "Wi-Fi connection service",
                DiagnosticStatus::Unknown,
                "The system service bus could not be reached, so PRYN could not check the Wi-Fi connection service.",
                Vec::new(),
            ));
            checks.push(DiagnosticCheck::new(
                "wifi.connection.state",
                "Wi-Fi connection",
                DiagnosticStatus::Unknown,
                "The Wi-Fi connection state could not be checked because the system service bus is unavailable.",
                Vec::new(),
            ));
        }
        WifiManagerProbe::QueryFailed => {
            checks.push(DiagnosticCheck::new(
                "wifi.connection_service.available",
                "Wi-Fi connection service",
                DiagnosticStatus::Unknown,
                "PRYN reached the system service bus but could not confirm the Wi-Fi connection service state.",
                Vec::new(),
            ));
            checks.push(DiagnosticCheck::new(
                "wifi.connection.state",
                "Wi-Fi connection",
                DiagnosticStatus::Unknown,
                "PRYN could not read the Wi-Fi connection state from the service.",
                Vec::new(),
            ));
        }
    }

    (checks, findings)
}

fn evaluate_networking_enabled(
    snapshot: &NetworkManagerSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    match snapshot.networking_enabled {
        Some(true) => checks.push(DiagnosticCheck::new(
            "wifi.networking.enabled",
            "System networking",
            DiagnosticStatus::Pass,
            "NetworkManager reports that networking is enabled.",
            vec![Evidence::new(
                "NetworkManager",
                "NetworkingEnabled is true.",
            )],
        )),
        Some(false) => {
            let summary = "NetworkManager reports that networking is disabled.";
            checks.push(DiagnosticCheck::new(
                "wifi.networking.enabled",
                "System networking",
                DiagnosticStatus::Problem,
                summary,
                vec![Evidence::new(
                    "NetworkManager",
                    "NetworkingEnabled is false.",
                )],
            ));
            findings.push(Finding::new(
                "WIFI-003",
                Severity::Medium,
                "Networking is turned off",
                summary,
                vec![Evidence::new(
                    "NetworkManager",
                    "The global networking switch is disabled.",
                )],
                vec![
                    "Check whether airplane mode or the system networking toggle is enabled.".to_owned(),
                    "Enable networking through your desktop's network settings if you expect to connect.".to_owned(),
                ],
            ));
        }
        None => checks.push(DiagnosticCheck::new(
            "wifi.networking.enabled",
            "System networking",
            DiagnosticStatus::Unknown,
            "PRYN could not read whether NetworkManager networking is enabled.",
            Vec::new(),
        )),
    }
}

fn evaluate_wireless_enabled(
    snapshot: &NetworkManagerSnapshot,
    kernel_wifi_interface_count: usize,
    rfkill_state: WifiRfkillState,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    match snapshot.wireless_enabled {
        Some(true) => checks.push(DiagnosticCheck::new(
            "wifi.radio.software_state",
            "Wi-Fi software switch",
            DiagnosticStatus::Pass,
            "NetworkManager reports that Wi-Fi is enabled.",
            vec![Evidence::new(
                "NetworkManager",
                "WirelessEnabled is true.",
            )],
        )),
        Some(false) => {
            let summary = "NetworkManager reports that Wi-Fi is switched off.";
            checks.push(DiagnosticCheck::new(
                "wifi.radio.software_state",
                "Wi-Fi software switch",
                DiagnosticStatus::Problem,
                summary,
                vec![Evidence::new(
                    "NetworkManager",
                    "WirelessEnabled is false.",
                )],
            ));
            // rfkill may already have reported the same software block. Keep
            // this check as corroborating evidence, but avoid duplicate findings.
            if rfkill_state.soft_blocked != Some(true) {
                findings.push(Finding::new(
                    "WIFI-004",
                    Severity::Medium,
                    "Wi-Fi is turned off",
                    summary,
                    vec![Evidence::new(
                        "NetworkManager",
                        "The Wi-Fi software switch is disabled.",
                    )],
                    vec![
                        "Turn Wi-Fi on using your desktop's network settings if you intend to use a wireless connection.".to_owned(),
                        "Rerun `pryn check wifi` after enabling Wi-Fi.".to_owned(),
                    ],
                ));
            }
        }
        None => checks.push(DiagnosticCheck::new(
            "wifi.radio.software_state",
            "Wi-Fi software switch",
            DiagnosticStatus::Unknown,
            "PRYN could not read the Wi-Fi software-switch state.",
            Vec::new(),
        )),
    }

    let has_wifi_device = !snapshot.wifi_devices.is_empty() || kernel_wifi_interface_count > 0;
    // The rfkill result is the more direct kernel observation. If it already
    // confirms a hard block, do not repeat the same diagnosis from NetworkManager.
    if rfkill_state.hard_blocked == Some(true) {
        return;
    }

    match (has_wifi_device, snapshot.wireless_hardware_enabled) {
        (true, Some(true)) => checks.push(DiagnosticCheck::new(
            "wifi.radio.hardware_state",
            "Wi-Fi hardware switch",
            DiagnosticStatus::Pass,
            "NetworkManager reports that the wireless hardware is enabled.",
            vec![Evidence::new(
                "NetworkManager",
                "WirelessHardwareEnabled is true.",
            )],
        )),
        (true, Some(false)) => {
            let summary = "NetworkManager reports that the wireless hardware is blocked.";
            checks.push(DiagnosticCheck::new(
                "wifi.radio.hardware_state",
                "Wi-Fi hardware switch",
                DiagnosticStatus::Problem,
                summary,
                vec![Evidence::new(
                    "NetworkManager",
                    "WirelessHardwareEnabled is false.",
                )],
            ));
            findings.push(Finding::new(
                "WIFI-005",
                Severity::High,
                "Wireless hardware is disabled",
                summary,
                vec![Evidence::new(
                    "NetworkManager",
                    "The hardware radio state is reported as disabled.",
                )],
                vec![
                    "Check the laptop's physical wireless switch or wireless keyboard shortcut.".to_owned(),
                    "Check firmware or BIOS wireless settings if no physical switch is available.".to_owned(),
                ],
            ));
        }
        (_, None) => checks.push(DiagnosticCheck::new(
            "wifi.radio.hardware_state",
            "Wi-Fi hardware switch",
            DiagnosticStatus::Unknown,
            "PRYN could not confirm the hardware radio state.",
            Vec::new(),
        )),
        (false, Some(_)) => checks.push(DiagnosticCheck::new(
            "wifi.radio.hardware_state",
            "Wi-Fi hardware switch",
            DiagnosticStatus::Unknown,
            "NetworkManager did not report a Wi-Fi device, so its hardware-switch state cannot be attributed to a detected adapter.",
            Vec::new(),
        )),
    }
}

fn evaluate_networkmanager_devices(
    snapshot: &NetworkManagerSnapshot,
    kernel_wifi_interface_count: usize,
    rfkill_state: WifiRfkillState,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    if !snapshot.devices_query_succeeded {
        checks.push(DiagnosticCheck::new(
            "wifi.connection.state",
            "Wi-Fi connection",
            DiagnosticStatus::Unknown,
            "PRYN could not retrieve the wireless-device list from NetworkManager.",
            Vec::new(),
        ));
        return;
    }

    if snapshot.wifi_devices.is_empty() {
        let summary = if kernel_wifi_interface_count > 0 {
            "The kernel exposes a wireless adapter, but NetworkManager did not report a Wi-Fi device. Another service may manage it, or the adapter may be unmanaged."
        } else {
            "NetworkManager did not report a Wi-Fi device. The hardware check could not confirm a wireless adapter either."
        };
        checks.push(DiagnosticCheck::new(
            "wifi.connection.state",
            "Wi-Fi connection",
            DiagnosticStatus::Unknown,
            summary,
            Vec::new(),
        ));
        return;
    }

    let managed_devices: Vec<&NetworkManagerWifiDevice> = snapshot
        .wifi_devices
        .iter()
        .filter(|device| {
            device.state != Some(NetworkManagerDeviceState::Unmanaged)
        })
        .collect();

    if managed_devices.is_empty() {
        checks.push(DiagnosticCheck::new(
            "wifi.connection.state",
            "Wi-Fi connection",
            DiagnosticStatus::NotApplicable,
            "NetworkManager sees the wireless adapter(s), but none is managed by NetworkManager. Another service or manual configuration may control them.",
            device_evidence(&snapshot.wifi_devices),
        ));
        return;
    }

    let active_devices: Vec<&NetworkManagerWifiDevice> = managed_devices
        .iter()
        .copied()
        .filter(|device| device.state == Some(NetworkManagerDeviceState::Activated))
        .collect();
    let failed_devices: Vec<&NetworkManagerWifiDevice> = managed_devices
        .iter()
        .copied()
        .filter(|device| device.state == Some(NetworkManagerDeviceState::Failed))
        .collect();
    let unavailable_devices: Vec<&NetworkManagerWifiDevice> = managed_devices
        .iter()
        .copied()
        .filter(|device| device.state == Some(NetworkManagerDeviceState::Unavailable))
        .collect();
    let disconnected_devices: Vec<&NetworkManagerWifiDevice> = managed_devices
        .iter()
        .copied()
        .filter(|device| device.state == Some(NetworkManagerDeviceState::Disconnected))
        .collect();
    let authentication_devices: Vec<&NetworkManagerWifiDevice> = managed_devices
        .iter()
        .copied()
        .filter(|device| {
            device.state == Some(NetworkManagerDeviceState::NeedsAuthentication)
        })
        .collect();

    let connection_is_enabled = snapshot.networking_enabled != Some(false)
        && snapshot.wireless_enabled != Some(false)
        && snapshot.wireless_hardware_enabled != Some(false)
        && rfkill_state.soft_blocked != Some(true)
        && rfkill_state.hard_blocked != Some(true);
    let is_transitioning = managed_devices.iter().any(|device| {
        matches!(
            device.state,
            Some(
                NetworkManagerDeviceState::Preparing
                    | NetworkManagerDeviceState::Configuring
                    | NetworkManagerDeviceState::IpConfiguration
                    | NetworkManagerDeviceState::IpCheck
                    | NetworkManagerDeviceState::WaitingForSecondary
                    | NetworkManagerDeviceState::Deactivating
            )
        )
    });

    let state_status = if !active_devices.is_empty() {
        DiagnosticStatus::Pass
    } else if !connection_is_enabled {
        DiagnosticStatus::NotApplicable
    } else if !failed_devices.is_empty() {
        DiagnosticStatus::Problem
    } else if !unavailable_devices.is_empty()
        || !disconnected_devices.is_empty()
        || !authentication_devices.is_empty()
        || is_transitioning
    {
        DiagnosticStatus::Warning
    } else {
        DiagnosticStatus::Unknown
    };

    let summaries = managed_devices
        .iter()
        .map(|device| {
            format!(
                "{}: {}",
                device.interface.as_deref().unwrap_or("Wi-Fi adapter"),
                device_state_label(device.state)
            )
        })
        .collect::<Vec<_>>();

    let summary = if !active_devices.is_empty() {
        let interfaces = active_devices
            .iter()
            .filter_map(|device| device.interface.as_deref())
            .collect::<Vec<_>>();
        let names = if interfaces.is_empty() {
            "the wireless adapter".to_owned()
        } else {
            interfaces.join(", ")
        };
        format!(
            "NetworkManager reports an active Wi-Fi connection on {names}. This confirms the manager's connection state, not internet reachability."
        )
    } else if !connection_is_enabled {
        "A Wi-Fi connection is not expected while system networking or the Wi-Fi radio is switched off.".to_owned()
    } else {
        format!("{}.", summaries.join("; "))
    };

    checks.push(DiagnosticCheck::new(
        "wifi.connection.state",
        "Wi-Fi connection",
        state_status,
        summary.clone(),
        device_evidence(&snapshot.wifi_devices),
    ));

    // A disconnected secondary adapter should not create a false alarm when
    // another adapter is already connected. Only raise state findings when no
    // managed adapter has an active connection.
    if active_devices.is_empty() && connection_is_enabled {
        if !failed_devices.is_empty() {
            findings.push(Finding::new(
                "WIFI-006",
                Severity::Medium,
                "A Wi-Fi connection attempt failed",
                format!(
                    "NetworkManager reports a failed connection state for {}.",
                    interface_names(&failed_devices)
                ),
                device_evidence_refs(&failed_devices),
                vec![
                    "Check the connection details and any password or authentication prompt.".to_owned(),
                    "Check recent NetworkManager logs for the reason the connection failed.".to_owned(),
                    "Try connecting again through your desktop's network controls.".to_owned(),
                ],
            ));
        } else if !unavailable_devices.is_empty() {
            let missing_dependency = unavailable_devices.iter().any(|device| {
                device.firmware_missing == Some(true) || device.plugin_missing == Some(true)
            });
            if !missing_dependency {
                let summary = "NetworkManager reports that a Wi-Fi adapter is unavailable. This can be caused by a disabled radio, a missing firmware component, or an unavailable device dependency.";
                findings.push(Finding::new(
                    "WIFI-007",
                    Severity::Medium,
                    "Wi-Fi adapter is unavailable",
                    summary,
                    device_evidence_refs(&unavailable_devices),
                    vec![
                        "Check the radio-block result above.".to_owned(),
                        "Check whether the device requires additional firmware or a driver package.".to_owned(),
                        "Review recent kernel and NetworkManager messages for more detail.".to_owned(),
                    ],
                ));
            }
        } else if !authentication_devices.is_empty() {
            findings.push(Finding::new(
                "WIFI-008",
                Severity::Medium,
                "Wi-Fi needs authentication",
                format!(
                    "NetworkManager is waiting for authentication on {}.",
                    interface_names(&authentication_devices)
                ),
                device_evidence_refs(&authentication_devices),
                vec![
                    "Open your desktop's network settings and complete the requested authentication.".to_owned(),
                    "Check that the saved network password or credentials are current.".to_owned(),
                ],
            ));
        } else if !disconnected_devices.is_empty() {
            findings.push(Finding::new(
                "WIFI-009",
                Severity::Low,
                "Wi-Fi is not connected",
                format!(
                    "NetworkManager reports that {} is ready but not connected to a network.",
                    interface_names(&disconnected_devices)
                ),
                device_evidence_refs(&disconnected_devices),
                vec![
                    "If you expect a connection, connect to a wireless network using your desktop's network controls.".to_owned(),
                    "Rerun `pryn check wifi` after connecting.".to_owned(),
                ],
            ));
        }
    }

    let firmware_missing = managed_devices
        .iter()
        .any(|device| device.firmware_missing == Some(true));
    let plugin_missing = managed_devices
        .iter()
        .any(|device| device.plugin_missing == Some(true));
    let dependency_values_known = managed_devices.iter().all(|device| {
        device.firmware_missing.is_some() && device.plugin_missing.is_some()
    });

    let (dependency_status, dependency_summary) = if firmware_missing || plugin_missing {
        (
            DiagnosticStatus::Warning,
            "NetworkManager reports a possible missing firmware component or device plugin.".to_owned(),
        )
    } else if dependency_values_known {
        (
            DiagnosticStatus::Pass,
            "NetworkManager did not report missing Wi-Fi firmware or device plugins.".to_owned(),
        )
    } else {
        (
            DiagnosticStatus::Unknown,
            "PRYN could not confirm whether all Wi-Fi firmware and device-plugin checks passed.".to_owned(),
        )
    };
    checks.push(DiagnosticCheck::new(
        "wifi.device_dependencies",
        "Wi-Fi firmware and support",
        dependency_status,
        dependency_summary.clone(),
        device_evidence(&snapshot.wifi_devices),
    ));

    if firmware_missing || plugin_missing {
        findings.push(Finding::new(
            "WIFI-010",
            Severity::Medium,
            "A Wi-Fi software dependency may be missing",
            dependency_summary,
            device_evidence(&snapshot.wifi_devices),
            vec![
                "Check the distribution's firmware packages for your wireless chipset.".to_owned(),
                "Check whether NetworkManager has the required device support installed.".to_owned(),
                "Rerun `pryn check wifi` after resolving the reported dependency.".to_owned(),
            ],
        ));
    }
}

fn evaluate_connectivity(
    snapshot: &NetworkManagerSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    if snapshot.connectivity_check_enabled == Some(false) {
        checks.push(DiagnosticCheck::new(
            "network.internet_connectivity",
            "System internet connectivity",
            DiagnosticStatus::NotApplicable,
            "NetworkManager's connectivity check is disabled. PRYN does not send its own internet test request.",
            Vec::new(),
        ));
        return;
    }

    if snapshot.connectivity_check_enabled != Some(true) {
        checks.push(DiagnosticCheck::new(
            "network.internet_connectivity",
            "System internet connectivity",
            DiagnosticStatus::Unknown,
            "PRYN could not confirm whether NetworkManager's connectivity check is enabled.",
            Vec::new(),
        ));
        return;
    }

    if snapshot.connectivity_check_available != Some(true) {
        checks.push(DiagnosticCheck::new(
            "network.internet_connectivity",
            "System internet connectivity",
            DiagnosticStatus::Unknown,
            "NetworkManager does not report a configured connectivity-check service, so PRYN cannot interpret its connectivity value.",
            vec![Evidence::new(
                "NetworkManager",
                format!(
                    "ConnectivityCheckAvailable is {:?}.",
                    snapshot.connectivity_check_available
                ),
            )],
        ));
        return;
    }

    let Some(connectivity) = snapshot.connectivity else {
        checks.push(DiagnosticCheck::new(
            "network.internet_connectivity",
            "System internet connectivity",
            DiagnosticStatus::Unknown,
            "NetworkManager's connectivity result is unavailable or has not been determined yet.",
            Vec::new(),
        ));
        return;
    };

    let (status, summary) = match connectivity {
        NetworkManagerConnectivity::Unknown | NetworkManagerConnectivity::Other(_) => (
            DiagnosticStatus::Unknown,
            "NetworkManager has not established a connectivity result.".to_owned(),
        ),
        NetworkManagerConnectivity::None => (
            DiagnosticStatus::Warning,
            "NetworkManager's last check reports that the host is not connected to a network with a default route to the internet. This is system-wide, not Wi-Fi-specific.".to_owned(),
        ),
        NetworkManagerConnectivity::Portal => (
            DiagnosticStatus::Warning,
            "NetworkManager's last check detected a possible captive portal. This result is system-wide, not Wi-Fi-specific, and may be stale.".to_owned(),
        ),
        NetworkManagerConnectivity::Limited => (
            DiagnosticStatus::Warning,
            "NetworkManager's last check reports limited system connectivity. It did not confirm full internet access, and the result may be stale.".to_owned(),
        ),
        NetworkManagerConnectivity::Full => (
            DiagnosticStatus::Pass,
            "NetworkManager's last connectivity check reports full system internet access. This result may be stale and is not specific to Wi-Fi. PRYN reads it without starting another check.".to_owned(),
        ),
    };

    checks.push(DiagnosticCheck::new(
        "network.internet_connectivity",
        "Internet connectivity",
        status,
        summary.clone(),
        vec![Evidence::new(
            "NetworkManager",
            format!("ConnectivityCheckEnabled and ConnectivityCheckAvailable are true; system connectivity state is {connectivity:?}."),
        )],
    ));

    match connectivity {
        NetworkManagerConnectivity::Portal => findings.push(Finding::new(
            "NET-001",
            Severity::Medium,
            "A network sign-in may be required",
            summary,
            vec![Evidence::new("NetworkManager", "Connectivity state is portal.")],
            vec![
                "Open a browser and complete the sign-in page for this network, if one is expected.".to_owned(),
                "After signing in, run `pryn check wifi` again.".to_owned(),
            ],
        )),
        NetworkManagerConnectivity::Limited => findings.push(Finding::new(
            "NET-002",
            Severity::Medium,
            "Internet access appears limited",
            summary,
            vec![Evidence::new("NetworkManager", "Connectivity state is limited.")],
            vec![
                "Check whether other devices on the same network can access the internet.".to_owned(),
                "Check the router, upstream connection, or network administrator for an outage.".to_owned(),
            ],
        )),
        NetworkManagerConnectivity::Unknown
        | NetworkManagerConnectivity::None
        | NetworkManagerConnectivity::Full
        | NetworkManagerConnectivity::Other(_) => {}
    }
}

fn device_state_label(state: Option<NetworkManagerDeviceState>) -> &'static str {
    match state {
        Some(NetworkManagerDeviceState::Unknown) | None => "state unavailable",
        Some(NetworkManagerDeviceState::Unmanaged) => "not managed by NetworkManager",
        Some(NetworkManagerDeviceState::Unavailable) => "unavailable",
        Some(NetworkManagerDeviceState::Disconnected) => "not connected",
        Some(NetworkManagerDeviceState::Preparing) => "preparing connection",
        Some(NetworkManagerDeviceState::Configuring) => "configuring connection",
        Some(NetworkManagerDeviceState::NeedsAuthentication) => "waiting for authentication",
        Some(NetworkManagerDeviceState::IpConfiguration) => "obtaining network settings",
        Some(NetworkManagerDeviceState::IpCheck) => "checking network connection",
        Some(NetworkManagerDeviceState::WaitingForSecondary) => "waiting for a secondary connection",
        Some(NetworkManagerDeviceState::Activated) => "connected to a network",
        Some(NetworkManagerDeviceState::Deactivating) => "disconnecting",
        Some(NetworkManagerDeviceState::Failed) => "connection failed",
        Some(NetworkManagerDeviceState::Other(_)) => "unrecognized connection state",
    }
}

fn interface_names(devices: &[&NetworkManagerWifiDevice]) -> String {
    devices
        .iter()
        .map(|device| device.interface.as_deref().unwrap_or("Wi-Fi adapter"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn device_evidence(devices: &[NetworkManagerWifiDevice]) -> Vec<Evidence> {
    devices
        .iter()
        .map(|device| {
            Evidence::new(
                "NetworkManager",
                format!(
                    "{}: state={}, firmware_missing={:?}, plugin_missing={:?}.",
                    device.interface.as_deref().unwrap_or("Wi-Fi adapter"),
                    device_state_label(device.state),
                    device.firmware_missing,
                    device.plugin_missing
                ),
            )
        })
        .collect()
}

fn device_evidence_refs(devices: &[&NetworkManagerWifiDevice]) -> Vec<Evidence> {
    devices
        .iter()
        .map(|device| {
            Evidence::new(
                "NetworkManager",
                format!(
                    "{}: {}.",
                    device.interface.as_deref().unwrap_or("Wi-Fi adapter"),
                    device_state_label(device.state)
                ),
            )
        })
        .collect()
}

fn ip_configuration_not_implemented() -> DiagnosticCheck {
    DiagnosticCheck::new(
        "wifi.ip_configuration",
        "Network settings",
        DiagnosticStatus::NotImplemented,
        "PRYN does not yet inspect IP addresses, routes, DNS, or internet reachability.",
        Vec::new(),
    )
}

#[cfg(test)]
mod tests {
    use super::{evaluate_snapshot, evaluate_network_manager};
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::network_manager::{
        NetworkManagerConnectivity, NetworkManagerDeviceState, NetworkManagerSnapshot,
        NetworkManagerWifiDevice, WifiManagerProbe,
    };
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

    fn active_network_manager() -> WifiManagerProbe {
        WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(true),
            connectivity_check_available: Some(true),
            connectivity: Some(NetworkManagerConnectivity::Full),
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Activated),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        })
    }

    #[test]
    fn wireless_link_does_not_claim_internet_connectivity() {
        let report = evaluate_snapshot(sample_snapshot(Some(true), false), active_network_manager());
        assert_eq!(report.subsystem, Subsystem::Wifi);
        assert!(report.checks.iter().any(|check| {
            check.id == "wifi.interface.carrier" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "wifi.connection.state" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "network.internet_connectivity" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "wifi.ip_configuration" && check.status == DiagnosticStatus::NotImplemented
        }));
        assert!(!report.checks.iter().any(|check| {
            check.id == "wifi.connection.state" && check.summary.to_lowercase().contains("internet")
        }));
    }

    #[test]
    fn blocked_radio_produces_a_problem_finding() {
        let report = evaluate_snapshot(sample_snapshot(Some(false), true), active_network_manager());
        assert!(report.findings.iter().any(|finding| finding.code == "WIFI-001"));
        assert_eq!(
            report.checks.iter().find(|check| check.id == "wifi.radio.block_state").map(|check| check.status),
            Some(DiagnosticStatus::Problem)
        );
    }

    #[test]
    fn no_wireless_interfaces_remains_unknown() {
        let report = evaluate_snapshot(
            WifiSnapshot {
                interfaces: Vec::new(),
                rfkill_devices: None,
                warnings: Vec::new(),
            },
            WifiManagerProbe::NoKnownManager,
        );
        assert_eq!(
            report.checks.iter().find(|check| check.id == "wifi.hardware.detected").map(|check| check.status),
            Some(DiagnosticStatus::Unknown)
        );
    }

    #[test]
    fn networkmanager_disconnected_device_is_a_warning_not_a_hardware_failure() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Disconnected),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (checks, findings) = evaluate_network_manager(probe, 1, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection.state").map(|check| check.status),
            Some(DiagnosticStatus::Warning)
        );
        assert!(findings.iter().any(|finding| finding.code == "WIFI-009"));
        assert!(!findings.iter().any(|finding| finding.code == "WIFI-006"));
    }

    #[test]
    fn active_adapter_prevents_false_alarm_for_disconnected_secondary_adapter() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![
                NetworkManagerWifiDevice {
                    interface: Some("wlp2s0".to_owned()),
                    state: Some(NetworkManagerDeviceState::Activated),
                    firmware_missing: Some(false),
                    plugin_missing: Some(false),
                },
                NetworkManagerWifiDevice {
                    interface: Some("wlp3s0".to_owned()),
                    state: Some(NetworkManagerDeviceState::Disconnected),
                    firmware_missing: Some(false),
                    plugin_missing: Some(false),
                },
            ],
            warnings: Vec::new(),
        });

        let (checks, findings) = evaluate_network_manager(probe, 2, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection.state").map(|check| check.status),
            Some(DiagnosticStatus::Pass)
        );
        assert!(!findings.iter().any(|finding| finding.code == "WIFI-009"));
    }

    #[test]
    fn software_disabled_wifi_produces_a_finding() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(false),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Unavailable),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (checks, findings) = evaluate_network_manager(probe, 1, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.radio.software_state").map(|check| check.status),
            Some(DiagnosticStatus::Problem)
        );
        assert!(findings.iter().any(|finding| finding.code == "WIFI-004"));
    }

    #[test]
    fn failed_connection_is_reported_with_evidence() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Failed),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (checks, findings) = evaluate_network_manager(probe, 1, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection.state").map(|check| check.status),
            Some(DiagnosticStatus::Problem)
        );
        assert!(findings.iter().any(|finding| finding.code == "WIFI-006"));
    }

    #[test]
    fn unmanaged_device_is_not_misreported_as_disconnected() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Unmanaged),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (checks, findings) = evaluate_network_manager(probe, 1, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection.state").map(|check| check.status),
            Some(DiagnosticStatus::NotApplicable)
        );
        assert!(findings.is_empty());
    }

    #[test]
    fn disabled_networkmanager_connectivity_check_is_not_reported_as_failure() {
        let snapshot = NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: Some(NetworkManagerConnectivity::Unknown),
            devices_query_succeeded: true,
            wifi_devices: Vec::new(),
            warnings: Vec::new(),
        };
        let mut checks = Vec::new();
        let mut findings = Vec::new();

        super::evaluate_connectivity(&snapshot, &mut checks, &mut findings);

        assert_eq!(checks[0].status, DiagnosticStatus::NotApplicable);
        assert!(findings.is_empty());
    }

    #[test]
    fn unavailable_networkmanager_connectivity_check_does_not_trust_stale_state() {
        let snapshot = NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(true),
            connectivity_check_available: Some(false),
            connectivity: Some(NetworkManagerConnectivity::Full),
            devices_query_succeeded: true,
            wifi_devices: Vec::new(),
            warnings: Vec::new(),
        };
        let mut checks = Vec::new();
        let mut findings = Vec::new();

        super::evaluate_connectivity(&snapshot, &mut checks, &mut findings);

        assert_eq!(checks[0].status, DiagnosticStatus::Unknown);
        assert!(checks[0].summary.contains("not currently available"));
        assert!(findings.is_empty());
    }

    #[test]
    fn missing_firmware_has_one_specific_finding_not_a_duplicate_unavailable_finding() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Unavailable),
                firmware_missing: Some(true),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (_, findings) = evaluate_network_manager(probe, 1, super::WifiRfkillState::default());

        assert!(findings.iter().any(|finding| finding.code == "WIFI-010"));
        assert!(!findings.iter().any(|finding| finding.code == "WIFI-007"));
    }

    #[test]
    fn networkmanager_portal_result_produces_a_finding() {
        let snapshot = NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(true),
            connectivity_check_available: Some(true),
            connectivity: Some(NetworkManagerConnectivity::Portal),
            devices_query_succeeded: true,
            wifi_devices: Vec::new(),
            warnings: Vec::new(),
        };
        let mut checks = Vec::new();
        let mut findings = Vec::new();

        super::evaluate_connectivity(&snapshot, &mut checks, &mut findings);

        assert_eq!(checks[0].status, DiagnosticStatus::Warning);
        assert!(findings.iter().any(|finding| finding.code == "NET-001"));
    }

    #[test]
    fn networkmanager_limited_connectivity_produces_a_finding() {
        let snapshot = NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(true),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(true),
            connectivity_check_available: Some(true),
            connectivity: Some(NetworkManagerConnectivity::Limited),
            devices_query_succeeded: true,
            wifi_devices: Vec::new(),
            warnings: Vec::new(),
        };
        let mut checks = Vec::new();
        let mut findings = Vec::new();

        super::evaluate_connectivity(&snapshot, &mut checks, &mut findings);

        assert_eq!(checks[0].status, DiagnosticStatus::Warning);
        assert!(findings.iter().any(|finding| finding.code == "NET-002"));
    }

    #[test]
    fn disabled_wifi_does_not_create_a_secondary_disconnected_finding() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(false),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Disconnected),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });

        let (checks, findings) = super::evaluate_network_manager(
            probe,
            1,
            super::WifiRfkillState::default(),
        );

        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection.state").map(|check| check.status),
            Some(DiagnosticStatus::NotApplicable)
        );
        assert!(findings.iter().any(|finding| finding.code == "WIFI-004"));
        assert!(!findings.iter().any(|finding| finding.code == "WIFI-009"));
    }

    #[test]
    fn rfkill_block_prevents_duplicate_networkmanager_radio_finding() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
            networking_enabled: Some(true),
            wireless_enabled: Some(false),
            wireless_hardware_enabled: Some(true),
            connectivity_check_enabled: Some(false),
            connectivity_check_available: Some(false),
            connectivity: None,
            devices_query_succeeded: true,
            wifi_devices: vec![NetworkManagerWifiDevice {
                interface: Some("wlp2s0".to_owned()),
                state: Some(NetworkManagerDeviceState::Unavailable),
                firmware_missing: Some(false),
                plugin_missing: Some(false),
            }],
            warnings: Vec::new(),
        });
        let rfkill = super::WifiRfkillState {
            soft_blocked: Some(true),
            hard_blocked: Some(false),
        };

        let (_, findings) = super::evaluate_network_manager(probe, 1, rfkill);

        assert!(!findings.iter().any(|finding| finding.code == "WIFI-004"));
        assert!(!findings.iter().any(|finding| finding.code == "WIFI-007"));
    }

    #[test]
    fn iwd_is_detected_but_not_claimed_as_supported_yet() {
        let (checks, _) = evaluate_network_manager(WifiManagerProbe::IwdDetected, 1, WifiRfkillState::default());
        assert_eq!(
            checks.iter().find(|check| check.id == "wifi.connection_service.available").map(|check| check.status),
            Some(DiagnosticStatus::NotImplemented)
        );
    }
}
