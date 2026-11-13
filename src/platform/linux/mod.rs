pub mod audio_server;
pub mod bluez;
pub mod network_manager;
pub mod pipewire;

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::model::SystemInfo;

#[derive(Clone, Debug)]
pub struct LinuxPaths {
    pub sys_class_net: PathBuf,
    pub sys_class_rfkill: PathBuf,
    pub sys_class_bluetooth: PathBuf,
    pub sys_class_sound: PathBuf,
}

impl Default for LinuxPaths {
    fn default() -> Self {
        Self {
            sys_class_net: PathBuf::from("/sys/class/net"),
            sys_class_rfkill: PathBuf::from("/sys/class/rfkill"),
            sys_class_bluetooth: PathBuf::from("/sys/class/bluetooth"),
            sys_class_sound: PathBuf::from("/sys/class/sound"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LinuxCollector {
    paths: LinuxPaths,
}

impl LinuxCollector {
    pub fn with_paths(paths: LinuxPaths) -> Self {
        Self { paths }
    }

    pub fn collect_wifi(&self) -> Result<WifiSnapshot, ProbeError> {
        let entries = read_directory(&self.paths.sys_class_net, false)?.unwrap_or_default();
        let mut interfaces = Vec::new();
        let mut warnings = Vec::new();

        for path in entries {
            let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                warnings.push(format!(
                    "Skipped a network interface with a non-UTF-8 name under {}",
                    self.paths.sys_class_net.display()
                ));
                continue;
            };

            let wireless_directory = path.join("wireless");
            let phy_link = path.join("phy80211");
            if !wireless_directory.is_dir() && !phy_link.exists() {
                continue;
            }

            interfaces.push(WifiInterface {
                name: name.to_owned(),
                operstate: read_optional_text(&path.join("operstate"), &mut warnings),
                carrier: read_optional_bool(&path.join("carrier"), &mut warnings),
                driver: read_driver_name(&path.join("device/driver"), &mut warnings),
            });
        }

        let rfkill_devices = collect_rfkill(&self.paths.sys_class_rfkill, &mut warnings)?;

        Ok(WifiSnapshot {
            interfaces,
            rfkill_devices,
            warnings,
        })
    }

    pub fn collect_bluetooth(&self) -> Result<BluetoothSnapshot, ProbeError> {
        let entries = read_directory(&self.paths.sys_class_bluetooth, true)?;
        let class_available = entries.is_some();
        let mut controllers = Vec::new();
        let mut warnings = Vec::new();

        if let Some(entries) = entries {
            for path in entries {
                let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                    warnings.push(format!(
                        "Skipped a Bluetooth controller with a non-UTF-8 name under {}",
                        self.paths.sys_class_bluetooth.display()
                    ));
                    continue;
                };

                if !name.starts_with("hci") {
                    continue;
                }

                controllers.push(BluetoothController {
                    name: name.to_owned(),
                    driver: read_driver_name(&path.join("device/driver"), &mut warnings),
                });
            }
        }

        let rfkill_devices = collect_rfkill(&self.paths.sys_class_rfkill, &mut warnings)?;

        Ok(BluetoothSnapshot {
            class_available,
            controllers,
            rfkill_devices,
            warnings,
        })
    }

    pub fn collect_audio(&self) -> Result<AudioSnapshot, ProbeError> {
        let entries = read_directory(&self.paths.sys_class_sound, true)?;
        let mut cards = Vec::new();
        let mut warnings = Vec::new();

        if let Some(entries) = &entries {
            for path in entries {
                let Some(name) = path.file_name().and_then(OsStr::to_str) else {
                    continue;
                };

                if !name.starts_with("card") {
                    continue;
                }

                cards.push(AudioCard {
                    kernel_name: name.to_owned(),
                    id: read_optional_text(&path.join("id"), &mut warnings),
                    driver: read_driver_name(&path.join("device/driver"), &mut warnings),
                });
            }
        }

        Ok(AudioSnapshot {
            class_available: entries.is_some(),
            cards,
            warnings,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WifiSnapshot {
    pub interfaces: Vec<WifiInterface>,
    pub rfkill_devices: Option<Vec<RfkillDevice>>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct WifiInterface {
    pub name: String,
    pub operstate: Option<String>,
    pub carrier: Option<bool>,
    pub driver: Option<String>,
}

#[derive(Clone, Debug)]
pub struct BluetoothSnapshot {
    pub class_available: bool,
    pub controllers: Vec<BluetoothController>,
    pub rfkill_devices: Option<Vec<RfkillDevice>>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct BluetoothController {
    pub name: String,
    pub driver: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AudioSnapshot {
    pub class_available: bool,
    pub cards: Vec<AudioCard>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AudioCard {
    pub kernel_name: String,
    pub id: Option<String>,
    pub driver: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RfkillDevice {
    pub name: Option<String>,
    pub kind: String,
    pub soft_blocked: Option<bool>,
    pub hard_blocked: Option<bool>,
}

#[derive(Debug, Error)]
#[error("could not inspect `{path}`: {source}")]
pub struct ProbeError {
    path: PathBuf,
    #[source]
    source: io::Error,
}

impl ProbeError {
    fn new(path: &Path, source: io::Error) -> Self {
        Self {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub fn collect_system_info() -> SystemInfo {
    let distribution = fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|contents| parse_os_release_value(&contents, "PRETTY_NAME"));

    let kernel_release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());

    SystemInfo {
        operating_system: "Linux".to_owned(),
        distribution,
        kernel_release,
        architecture: std::env::consts::ARCH.to_owned(),
    }
}

fn parse_os_release_value(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let (candidate, value) = line.split_once('=')?;
        if candidate != key {
            return None;
        }

        let normalized = value.trim().trim_matches('"').trim_matches('\'');
        if normalized.is_empty() {
            None
        } else {
            Some(normalized.to_owned())
        }
    })
}

fn read_directory(
    path: &Path,
    missing_is_optional: bool,
) -> Result<Option<Vec<PathBuf>>, ProbeError> {
    let directory = match fs::read_dir(path) {
        Ok(directory) => directory,
        Err(source) if missing_is_optional && source.kind() == io::ErrorKind::NotFound => {
            return Ok(None);
        }
        Err(source) => return Err(ProbeError::new(path, source)),
    };

    let mut entries = Vec::new();
    for entry in directory {
        match entry {
            Ok(entry) => entries.push(entry.path()),
            Err(source) => return Err(ProbeError::new(path, source)),
        }
    }

    entries.sort();
    Ok(Some(entries))
}

fn collect_rfkill(
    root: &Path,
    warnings: &mut Vec<String>,
) -> Result<Option<Vec<RfkillDevice>>, ProbeError> {
    let Some(entries) = read_directory(root, true)? else {
        return Ok(None);
    };

    let mut devices = Vec::new();
    for path in entries {
        let kind = read_optional_text(&path.join("type"), warnings);
        let Some(kind) = kind else {
            continue;
        };

        devices.push(RfkillDevice {
            name: read_optional_text(&path.join("name"), warnings),
            kind,
            soft_blocked: read_optional_bool(&path.join("soft"), warnings),
            hard_blocked: read_optional_bool(&path.join("hard"), warnings),
        });
    }

    Ok(Some(devices))
}

fn read_optional_text(path: &Path, warnings: &mut Vec<String>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(value) => {
            let value = value.trim().to_owned();
            if value.is_empty() { None } else { Some(value) }
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            warnings.push(format!(
                "Could not read {} ({:?})",
                path.display(),
                source.kind()
            ));
            None
        }
    }
}

fn read_optional_bool(path: &Path, warnings: &mut Vec<String>) -> Option<bool> {
    let value = read_optional_text(path, warnings)?;
    match value.as_str() {
        "0" => Some(false),
        "1" => Some(true),
        _ => {
            warnings.push(format!("Unexpected boolean value in {}", path.display()));
            None
        }
    }
}

fn read_driver_name(path: &Path, warnings: &mut Vec<String>) -> Option<String> {
    match fs::read_link(path) {
        Ok(target) => target
            .file_name()
            .and_then(OsStr::to_str)
            .map(str::to_owned),
        Err(source) if source.kind() == io::ErrorKind::NotFound => None,
        Err(source) => {
            warnings.push(format!(
                "Could not inspect driver link {} ({:?})",
                path.display(),
                source.kind()
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{LinuxCollector, LinuxPaths};
    use std::fs;
    use std::os::unix::fs::symlink;
    use tempfile::TempDir;

    fn fixture_paths(root: &TempDir) -> LinuxPaths {
        LinuxPaths {
            sys_class_net: root.path().join("sys/class/net"),
            sys_class_rfkill: root.path().join("sys/class/rfkill"),
            sys_class_bluetooth: root.path().join("sys/class/bluetooth"),
            sys_class_sound: root.path().join("sys/class/sound"),
        }
    }

    #[test]
    fn collects_wireless_interface_and_rfkill_state() {
        let root = tempfile::tempdir().expect("temporary directory should be created");
        let paths = fixture_paths(&root);
        let interface = paths.sys_class_net.join("wlp2s0");
        let driver = root.path().join("sys/bus/pci/drivers/iwlwifi");
        fs::create_dir_all(interface.join("wireless")).expect("wireless directory");
        fs::create_dir_all(interface.join("device")).expect("device directory");
        fs::create_dir_all(&driver).expect("driver directory");
        symlink(&driver, interface.join("device/driver")).expect("driver symlink");
        fs::write(interface.join("operstate"), "up\n").expect("operstate");
        fs::write(interface.join("carrier"), "1\n").expect("carrier");

        let rfkill = paths.sys_class_rfkill.join("rfkill0");
        fs::create_dir_all(&rfkill).expect("rfkill directory");
        fs::write(rfkill.join("type"), "wlan\n").expect("rfkill type");
        fs::write(rfkill.join("name"), "phy0\n").expect("rfkill name");
        fs::write(rfkill.join("soft"), "0\n").expect("rfkill soft");
        fs::write(rfkill.join("hard"), "0\n").expect("rfkill hard");

        let snapshot = LinuxCollector::with_paths(paths)
            .collect_wifi()
            .expect("Wi-Fi collection should succeed");

        assert_eq!(snapshot.interfaces.len(), 1);
        assert_eq!(snapshot.interfaces[0].name, "wlp2s0");
        assert_eq!(snapshot.interfaces[0].operstate.as_deref(), Some("up"));
        assert_eq!(snapshot.interfaces[0].carrier, Some(true));
        assert_eq!(snapshot.interfaces[0].driver.as_deref(), Some("iwlwifi"));
        assert_eq!(snapshot.rfkill_devices.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn missing_optional_bluetooth_and_sound_classes_are_not_fatal() {
        let root = tempfile::tempdir().expect("temporary directory should be created");
        let paths = fixture_paths(&root);
        let collector = LinuxCollector::with_paths(paths);

        let bluetooth = collector
            .collect_bluetooth()
            .expect("missing Bluetooth class should be handled");
        let audio = collector
            .collect_audio()
            .expect("missing sound class should be handled");

        assert!(!bluetooth.class_available);
        assert!(bluetooth.controllers.is_empty());
        assert!(!audio.class_available);
        assert!(audio.cards.is_empty());
    }

    #[test]
    fn collects_bluetooth_controller_and_audio_card() {
        let root = tempfile::tempdir().expect("temporary directory should be created");
        let paths = fixture_paths(&root);

        let controller = paths.sys_class_bluetooth.join("hci0");
        fs::create_dir_all(controller.join("device")).expect("controller device directory");
        fs::create_dir_all(paths.sys_class_sound.join("card0")).expect("audio card directory");
        fs::write(paths.sys_class_sound.join("card0/id"), "PCH\n").expect("audio card id");

        let collector = LinuxCollector::with_paths(paths);
        let bluetooth = collector
            .collect_bluetooth()
            .expect("Bluetooth collection should succeed");
        let audio = collector
            .collect_audio()
            .expect("audio collection should succeed");

        assert!(bluetooth.class_available);
        assert_eq!(bluetooth.controllers.len(), 1);
        assert_eq!(bluetooth.controllers[0].name, "hci0");
        assert_eq!(audio.cards.len(), 1);
        assert_eq!(audio.cards[0].id.as_deref(), Some("PCH"));
    }
}
