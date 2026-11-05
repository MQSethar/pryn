use crate::model::{DiagnosticCheck, DiagnosticStatus, Evidence, Subsystem, SubsystemReport};
use crate::platform::linux::{AudioSnapshot, LinuxCollector};

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    match collector.collect_audio() {
        Ok(snapshot) => evaluate_snapshot(snapshot),
        Err(error) => SubsystemReport::new(
            Subsystem::Audio,
            vec![DiagnosticCheck::new(
                "audio.inventory.available",
                "Audio hardware inventory",
                DiagnosticStatus::Unknown,
                "PRYN could not inspect the kernel sound class.",
                vec![Evidence::new("sysfs:/sys/class/sound", error.to_string())],
            )],
            Vec::new(),
        ),
    }
}

fn evaluate_snapshot(snapshot: AudioSnapshot) -> SubsystemReport {
    let mut checks = Vec::new();

    let hardware_check = if !snapshot.class_available {
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Unknown,
            "The kernel sound class is not exposed. This does not prove that the computer has no audio hardware.",
            vec![Evidence::new(
                "sysfs:/sys/class/sound",
                "The sound class directory is unavailable.",
            )],
        )
    } else if snapshot.cards.is_empty() {
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Unknown,
            "No ALSA sound card was discovered in sysfs.",
            vec![Evidence::new(
                "sysfs:/sys/class/sound",
                "The sound class exists but no `card*` entries were found.",
            )],
        )
    } else {
        let card_names = snapshot
            .cards
            .iter()
            .map(|card| match &card.id {
                Some(id) => format!("{} ({id})", card.kernel_name),
                None => card.kernel_name.clone(),
            })
            .collect::<Vec<_>>();
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Pass,
            format!(
                "Detected {} ALSA sound card(s): {}.",
                card_names.len(),
                card_names.join(", ")
            ),
            snapshot
                .cards
                .iter()
                .map(|card| {
                    Evidence::new(
                        format!("sysfs:/sys/class/sound/{}", card.kernel_name),
                        match &card.id {
                            Some(id) => format!("ALSA card identifier: {id}"),
                            None => {
                                "ALSA card entry discovered; identifier unavailable.".to_owned()
                            }
                        },
                    )
                })
                .collect(),
        )
    };
    checks.push(hardware_check);

    let driver_count = snapshot
        .cards
        .iter()
        .filter(|card| card.driver.is_some())
        .count();
    let driver_status = if !snapshot.cards.is_empty() && driver_count == snapshot.cards.len() {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Unknown
    };
    checks.push(DiagnosticCheck::new(
        "audio.driver.association",
        "Device driver",
        driver_status,
        if driver_count == 0 {
            "No audio card driver association was exposed through sysfs.".to_owned()
        } else {
            format!(
                "Driver associations were exposed for {driver_count} of {} audio card(s).",
                snapshot.cards.len()
            )
        },
        snapshot
            .cards
            .iter()
            .filter_map(|card| {
                card.driver.as_ref().map(|driver| {
                    Evidence::new(
                        format!("sysfs:/class/sound/{}/device/driver", card.kernel_name),
                        format!("Driver: {driver}"),
                    )
                })
            })
            .collect(),
    ));

    checks.push(DiagnosticCheck::new(
        "audio.server.state",
        "Audio service",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet confirm whether your desktop audio service is running.",
        Vec::new(),
    ));
    checks.push(DiagnosticCheck::new(
        "audio.routing.state",
        "Audio inputs and outputs",
        DiagnosticStatus::NotImplemented,
        "PRYN cannot yet check the selected speakers, microphone, volume, mute state, or app audio.",
        Vec::new(),
    ));

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "audio.collection.completeness",
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

    SubsystemReport::new(Subsystem::Audio, checks, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::evaluate_snapshot;
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::{AudioCard, AudioSnapshot};

    #[test]
    fn alsa_card_is_detected_without_claiming_audio_playback_works() {
        let report = evaluate_snapshot(AudioSnapshot {
            class_available: true,
            cards: vec![AudioCard {
                kernel_name: "card0".to_owned(),
                id: Some("PCH".to_owned()),
                driver: Some("snd_hda_intel".to_owned()),
            }],
            warnings: Vec::new(),
        });

        assert_eq!(report.subsystem, Subsystem::Audio);
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.hardware.detected" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.server.state" && check.status == DiagnosticStatus::NotImplemented
        }));
    }
}
