mod audio;
mod bluetooth;
mod wifi;

use crate::{
    model::{Subsystem, SubsystemReport},
    platform::linux::LinuxPlatform,
};

pub use audio::AudioModule;
pub use bluetooth::BluetoothModule;
pub use wifi::WifiModule;

/// A built-in diagnostic subsystem.
///
/// Modules consume normalized access through LinuxPlatform and return data;
/// they do not print output or modify the host system.
pub trait DiagnosticModule: Sync {
    fn subsystem(&self) -> Subsystem;
    fn diagnose(&self, platform: &LinuxPlatform) -> SubsystemReport;
}
