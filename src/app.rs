use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{DiagnosticReport, Subsystem};
use crate::modules::{audio, bluetooth, wifi};
use crate::platform::linux::{LinuxCollector, collect_system_info};

pub fn check(target: Option<Subsystem>) -> DiagnosticReport {
    tracing::info!(?target, "starting system checks");
    let collector = LinuxCollector::default();
    let mut subsystems = Vec::new();

    if target.is_none() || target == Some(Subsystem::Wifi) {
        subsystems.push(wifi::check(&collector));
    }
    if target.is_none() || target == Some(Subsystem::Bluetooth) {
        subsystems.push(bluetooth::check(&collector));
    }
    if target.is_none() || target == Some(Subsystem::Audio) {
        subsystems.push(audio::check(&collector));
    }

    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    DiagnosticReport::new(generated_at_unix, collect_system_info(), subsystems)
}
