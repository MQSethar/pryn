use crate::{
    model::{CheckResult, CheckStatus, Evidence, Severity, Subsystem, SubsystemReport},
    modules::DiagnosticModule,
    platform::linux::LinuxPlatform,
};

/// Initial audio diagnostics backed by the ALSA kernel inventory.
pub struct AudioModule;

impl DiagnosticModule for AudioModule {
    fn subsystem(&self) -> Subsystem {
        Subsystem::Audio
    }

    fn diagnose(&self, platform: &LinuxPlatform) -> SubsystemReport {
        let checks = vec![check_alsa_cards(platform), check_audio_server_state()];
        SubsystemReport::new(Subsystem::Audio, checks)
    }
}

fn check_alsa_cards(platform: &LinuxPlatform) -> CheckResult {
    match platform.alsa_cards() {
        Ok(cards) if !cards.is_empty() => {
            let mut result = CheckResult::new(
                "AUDIO.ALSA_CARDS",
                "ALSA sound-card inventory",
                CheckStatus::Pass,
                Severity::Info,
                format!("ALSA reports {count} sound card(s).", count = cards.len()),
            );
            for card in cards {
                result
                    .evidence
                    .push(Evidence::new("/proc/asound/cards", card));
            }
            result
        }
        Ok(_) => CheckResult::new(
            "AUDIO.ALSA_CARDS",
            "ALSA sound-card inventory",
            CheckStatus::Warning,
            Severity::Medium,
            "ALSA did not report any sound cards.",
        )
        .with_evidence(Evidence::new(
            "/proc/asound/cards",
            "The inventory was readable but contained no parseable sound-card entries.",
        ))
        .with_recommendation(
            "Check whether an audio device is present and whether its kernel driver is loaded. Some virtual or non-ALSA audio setups may not expose a hardware card here.",
        ),
        Err(error) => CheckResult::new(
            "AUDIO.ALSA_CARDS",
            "ALSA sound-card inventory",
            CheckStatus::Unknown,
            Severity::Info,
            "PRYN could not read the ALSA sound-card inventory.",
        )
        .with_evidence(Evidence::new("/proc/asound/cards", error.to_string())),
    }
}

fn check_audio_server_state() -> CheckResult {
    CheckResult::new(
        "AUDIO.SERVER",
        "Userspace audio server",
        CheckStatus::Unknown,
        Severity::Info,
        "PipeWire and PulseAudio server state is not inspected in this initial release.",
    )
    .with_recommendation(
        "A later provider will inspect PipeWire and PulseAudio-compatible interfaces, then report available devices, default routes, mute state, and streams.",
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::tempdir;

    use crate::{
        model::CheckStatus,
        modules::{AudioModule, DiagnosticModule},
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
    fn detects_alsa_card_from_proc_fixture() {
        let root = tempdir().expect("temporary directory should be created");
        let cards_path = root.path().join("proc/asound/cards");
        fs::create_dir_all(cards_path.parent().expect("cards has a parent"))
            .expect("ALSA fixture directory should be created");
        fs::write(
            cards_path,
            " 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n                      HDA Intel PCH at 0x... irq 125\n",
        )
        .expect("ALSA fixture should be written");

        let report = AudioModule.diagnose(&platform(root.path()));
        assert_eq!(report.checks[0].status, CheckStatus::Pass);
        assert_eq!(report.checks[0].evidence.len(), 1);
    }

    #[test]
    fn readable_empty_alsa_inventory_is_a_warning() {
        let root = tempdir().expect("temporary directory should be created");
        let cards_path = root.path().join("proc/asound/cards");
        fs::create_dir_all(cards_path.parent().expect("cards has a parent"))
            .expect("ALSA fixture directory should be created");
        fs::write(cards_path, "--- no soundcards ---\n")
            .expect("empty ALSA inventory should be written");

        let report = AudioModule.diagnose(&platform(root.path()));
        assert_eq!(report.checks[0].status, CheckStatus::Warning);
    }

    #[test]
    fn missing_alsa_inventory_is_unknown() {
        let root = tempdir().expect("temporary directory should be created");
        let report = AudioModule.diagnose(&platform(root.path()));
        assert_eq!(report.checks[0].status, CheckStatus::Unknown);
    }
}
