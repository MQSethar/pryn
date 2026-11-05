use crate::model::{DiagnosticCheck, DiagnosticStatus, Evidence, Finding, Severity, Subsystem};
use crate::platform::linux::RfkillDevice;

pub fn evaluate_radio_block(
    subsystem: Subsystem,
    radio_type: &str,
    check_id: &str,
    finding_code: &str,
    devices: Option<&[RfkillDevice]>,
) -> (DiagnosticCheck, Option<Finding>) {
    let Some(devices) = devices else {
        return (
            DiagnosticCheck::new(
                check_id,
                "Radio block state",
                DiagnosticStatus::Unknown,
                "The kernel did not expose rfkill state. Radio availability cannot be confirmed.",
                vec![Evidence::new(
                    "sysfs:/sys/class/rfkill",
                    "The rfkill class directory is unavailable.",
                )],
            ),
            None,
        );
    };

    let matching: Vec<&RfkillDevice> = devices
        .iter()
        .filter(|device| device.kind == radio_type)
        .collect();

    if matching.is_empty() {
        return (
            DiagnosticCheck::new(
                check_id,
                "Radio block state",
                DiagnosticStatus::Unknown,
                format!(
                    "No {radio_type} rfkill entries were reported; this does not prove the radio is enabled."
                ),
                vec![Evidence::new(
                    "sysfs:/sys/class/rfkill",
                    format!("No rfkill entries of type `{radio_type}` were found."),
                )],
            ),
            None,
        );
    }

    let blocked: Vec<&RfkillDevice> = matching
        .iter()
        .copied()
        .filter(|device| device.soft_blocked == Some(true) || device.hard_blocked == Some(true))
        .collect();

    if !blocked.is_empty() {
        let hard_blocked = blocked
            .iter()
            .any(|device| device.hard_blocked == Some(true));
        let details = blocked
            .iter()
            .map(|device| {
                format!(
                    "{}: soft_blocked={:?}, hard_blocked={:?}",
                    device.name.as_deref().unwrap_or("unnamed"),
                    device.soft_blocked,
                    device.hard_blocked
                )
            })
            .collect::<Vec<_>>();
        let affected = format!("Blocked radio evidence: {}.", details.join("; "));
        let block_kind = if hard_blocked {
            "A hardware-level block is active. Software may not be able to override it."
        } else {
            "A software-level radio block is active."
        };
        let finding = Finding::new(
            finding_code,
            if hard_blocked {
                Severity::High
            } else {
                Severity::Medium
            },
            format!("{subsystem} radio is blocked"),
            format!("{affected} {block_kind}"),
            vec![Evidence::new("sysfs:/sys/class/rfkill", details.join("; "))],
            if hard_blocked {
                vec![
                    "Check the laptop's physical wireless switch or hardware key.".to_owned(),
                    "Check firmware or BIOS settings if the hardware switch is not available."
                        .to_owned(),
                    "Rerun PRYN after the hardware-level block is cleared.".to_owned(),
                ]
            } else {
                vec![
                    "Check the desktop's radio toggle or the network settings.".to_owned(),
                    "If you intentionally enabled airplane mode, no action is required.".to_owned(),
                    "Rerun PRYN after changing the radio state.".to_owned(),
                ]
            },
        );
        return (
            DiagnosticCheck::new(
                check_id,
                "Radio block state",
                DiagnosticStatus::Problem,
                format!("{} {}", affected, block_kind),
                vec![Evidence::new(
                    "sysfs:/sys/class/rfkill",
                    format!(
                        "{} matching rfkill entr{} inspected.",
                        matching.len(),
                        if matching.len() == 1 { "y" } else { "ies" }
                    ),
                )],
            ),
            Some(finding),
        );
    }

    let all_states_known = matching
        .iter()
        .all(|device| device.soft_blocked.is_some() && device.hard_blocked.is_some());

    if all_states_known {
        (
            DiagnosticCheck::new(
                check_id,
                "Radio block state",
                DiagnosticStatus::Pass,
                format!("No {radio_type} rfkill entry reports a soft or hard block."),
                vec![Evidence::new(
                    "sysfs:/sys/class/rfkill",
                    format!(
                        "{} matching rfkill entr{} report both block flags as clear.",
                        matching.len(),
                        if matching.len() == 1 { "y" } else { "ies" }
                    ),
                )],
            ),
            None,
        )
    } else {
        (
            DiagnosticCheck::new(
                check_id,
                "Radio block state",
                DiagnosticStatus::Unknown,
                "Some rfkill block flags could not be read. The radio state is not fully known.",
                vec![Evidence::new(
                    "sysfs:/sys/class/rfkill",
                    "At least one soft or hard block value was unavailable.",
                )],
            ),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate_radio_block;
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::RfkillDevice;

    fn unblocked_wlan() -> RfkillDevice {
        RfkillDevice {
            name: Some("phy0".to_owned()),
            kind: "wlan".to_owned(),
            soft_blocked: Some(false),
            hard_blocked: Some(false),
        }
    }

    #[test]
    fn missing_rfkill_information_is_unknown_not_pass() {
        let (check, finding) = evaluate_radio_block(
            Subsystem::Wifi,
            "wlan",
            "wifi.radio.block_state",
            "WIFI-001",
            None,
        );
        assert_eq!(check.status, DiagnosticStatus::Unknown);
        assert!(finding.is_none());
    }

    #[test]
    fn unblocked_radio_passes() {
        let (check, finding) = evaluate_radio_block(
            Subsystem::Wifi,
            "wlan",
            "wifi.radio.block_state",
            "WIFI-001",
            Some(&[unblocked_wlan()]),
        );
        assert_eq!(check.status, DiagnosticStatus::Pass);
        assert!(finding.is_none());
    }

    #[test]
    fn hard_block_creates_a_finding() {
        let device = RfkillDevice {
            name: Some("phy0".to_owned()),
            kind: "wlan".to_owned(),
            soft_blocked: Some(false),
            hard_blocked: Some(true),
        };
        let (check, finding) = evaluate_radio_block(
            Subsystem::Wifi,
            "wlan",
            "wifi.radio.block_state",
            "WIFI-001",
            Some(&[device]),
        );
        assert_eq!(check.status, DiagnosticStatus::Problem);
        assert_eq!(
            finding.map(|item| item.severity),
            Some(crate::model::Severity::High)
        );
    }
}
