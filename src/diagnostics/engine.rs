use crate::{
    model::{DiagnosticReport, Subsystem},
    modules::{AudioModule, BluetoothModule, DiagnosticModule, WifiModule},
    platform::linux::LinuxPlatform,
};

/// Run the selected built-in diagnostic modules and assemble one report.
///
/// Collectors are deliberately read-only. The engine owns orchestration and
/// report assembly; subsystem modules own their checks and interpretation.
pub fn run(requested_subsystem: Option<Subsystem>) -> DiagnosticReport {
    tracing::debug!(?requested_subsystem, "starting diagnostic run");
    let platform = LinuxPlatform::default();
    let system = platform.system_info();

    let wifi = WifiModule;
    let bluetooth = BluetoothModule;
    let audio = AudioModule;
    let modules: [&dyn DiagnosticModule; 3] = [&wifi, &bluetooth, &audio];

    let subsystem_reports = modules
        .iter()
        .filter(|module| {
            requested_subsystem.is_none_or(|requested| module.subsystem() == requested)
        })
        .map(|module| module.diagnose(&platform))
        .collect();

    DiagnosticReport::new(system, subsystem_reports)
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::model::Subsystem;

    #[test]
    fn runs_all_three_initial_subsystems() {
        let report = run(None);
        assert_eq!(report.subsystems.len(), 3);
    }

    #[test]
    fn runs_only_the_requested_subsystem() {
        let report = run(Some(Subsystem::Wifi));
        assert_eq!(report.subsystems.len(), 1);
        assert_eq!(report.subsystems[0].subsystem, Subsystem::Wifi);
    }
}
