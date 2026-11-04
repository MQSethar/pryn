use std::{
    collections::BTreeMap,
    fs,
    io,
    path::{Path, PathBuf},
};

use crate::model::SystemInfo;

/// Filesystem roots used by Linux collectors.
///
/// Keeping roots explicit makes collectors testable with temporary fixtures and
/// avoids hard-coding host paths throughout diagnostic modules.
#[derive(Debug, Clone)]
pub struct LinuxPaths {
    pub etc_root: PathBuf,
    pub proc_root: PathBuf,
    pub sys_root: PathBuf,
}

impl Default for LinuxPaths {
    fn default() -> Self {
        Self {
            etc_root: PathBuf::from("/etc"),
            proc_root: PathBuf::from("/proc"),
            sys_root: PathBuf::from("/sys"),
        }
    }
}

/// Read-only access to the Linux interfaces currently used by PRYN.
#[derive(Debug, Clone, Default)]
pub struct LinuxPlatform {
    paths: LinuxPaths,
}

impl LinuxPlatform {
    pub fn new(paths: LinuxPaths) -> Self {
        Self { paths }
    }

    pub fn system_info(&self) -> SystemInfo {
        let os_release_path = self.paths.etc_root.join("os-release");
        let os_values = match fs::read_to_string(&os_release_path) {
            Ok(contents) => parse_os_release(&contents),
            Err(_) => BTreeMap::new(),
        };

        let kernel_release = read_trimmed(&self.paths.proc_root.join("sys/kernel/osrelease"));

        SystemInfo {
            os_id: os_values.get("ID").cloned(),
            os_name: os_values.get("NAME").cloned(),
            os_pretty_name: os_values.get("PRETTY_NAME").cloned(),
            os_version_id: os_values.get("VERSION_ID").cloned(),
            kernel_release,
            architecture: std::env::consts::ARCH.to_owned(),
        }
    }

    /// Find wireless network interfaces and kernel wireless PHY devices.
    pub fn wifi_devices(&self) -> io::Result<Vec<String>> {
        let network_root = self.paths.sys_root.join("class/net");
        let phy_root = self.paths.sys_root.join("class/ieee80211");
        let mut results = Vec::new();
        let mut inspected_any = false;
        let mut first_error = None;

        match fs::read_dir(&network_root) {
            Ok(entries) => {
                inspected_any = true;
                for entry in entries {
                    let entry = entry?;
                    let interface_path = entry.path();
                    if interface_path.join("wireless").exists()
                        || interface_path.join("phy80211").exists()
                    {
                        let name = entry.file_name();
                        results.push(format!("interface {name}", name = name.to_string_lossy()));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => first_error = Some(error),
        }

        match fs::read_dir(&phy_root) {
            Ok(entries) => {
                inspected_any = true;
                for entry in entries {
                    let entry = entry?;
                    let name = entry.file_name();
                    results.push(format!("PHY {name}", name = name.to_string_lossy()));
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        results.sort();
        results.dedup();

        if !results.is_empty() {
            Ok(results)
        } else if let Some(error) = first_error {
            Err(error)
        } else if inspected_any {
            Ok(results)
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "neither /sys/class/net nor /sys/class/ieee80211 is available",
            ))
        }
    }

    /// Read rfkill state for a kernel rfkill type (wlan or bluetooth).
    pub fn rfkill_states(&self, expected_type: &str) -> io::Result<Vec<RfkillState>> {
        let root = self.paths.sys_root.join("class/rfkill");
        let entries = fs::read_dir(&root)?;
        let mut states = Vec::new();

        for entry in entries {
            let entry = entry?;
            let device_path = entry.path();
            let device_type = read_trimmed(&device_path.join("type"));

            if device_type.as_deref() != Some(expected_type) {
                continue;
            }

            let name = entry.file_name().to_string_lossy().into_owned();
            let soft_blocked = read_boolean(&device_path.join("soft"))?;
            let hard_blocked = read_boolean(&device_path.join("hard"))?;

            states.push(RfkillState {
                name,
                soft_blocked,
                hard_blocked,
            });
        }

        states.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(states)
    }

    /// Find Bluetooth controllers exposed by the kernel.
    pub fn bluetooth_controllers(&self) -> io::Result<Vec<String>> {
        let root = self.paths.sys_root.join("class/bluetooth");
        let entries = fs::read_dir(&root)?;
        let mut controllers = Vec::new();

        for entry in entries {
            let entry = entry?;
            controllers.push(entry.file_name().to_string_lossy().into_owned());
        }

        controllers.sort();
        Ok(controllers)
    }

    /// Read ALSA's kernel-exposed sound-card inventory.
    pub fn alsa_cards(&self) -> io::Result<Vec<String>> {
        let path = self.paths.proc_root.join("asound/cards");
        let contents = fs::read_to_string(path)?;
        Ok(parse_alsa_cards(&contents))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RfkillState {
    pub name: String,
    pub soft_blocked: bool,
    pub hard_blocked: bool,
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|contents| contents.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn read_boolean(path: &Path) -> io::Result<bool> {
    let value = fs::read_to_string(path)?;
    match value.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected 0 or 1 in {path}, found {other:?}"),
            path = path.display()
        )),
    }
}

fn parse_os_release(contents: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty()
            || !key.chars().all(|character| {
                character.is_ascii_uppercase()
                    || character.is_ascii_digit()
                    || character == '_'
            })
        {
            continue;
        }

        values.insert(key.to_owned(), unquote_os_release_value(raw_value.trim()));
    }

    values
}

fn unquote_os_release_value(value: &str) -> String {
    if value.len() >= 2 {
        let first = value.as_bytes()[0];
        let last = value.as_bytes()[value.len() - 1];

        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            let inner = &value[1..value.len() - 1];
            return inner.replace("\\\"", "\"").replace("\\\\", "\\");
        }
    }

    value.to_owned()
}

fn parse_alsa_cards(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let (number, _) = trimmed.split_once('[')?;
            number
                .trim()
                .parse::<u32>()
                .ok()
                .map(|_| trimmed.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_alsa_cards, parse_os_release};

    #[test]
    fn parses_os_release_without_executing_shell_syntax() {
        let parsed = parse_os_release(
            "# comment\nID=ubuntu\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\nUNSAFE=$(touch /tmp/pryn-test)\n",
        );

        assert_eq!(parsed.get("ID").map(String::as_str), Some("ubuntu"));
        assert_eq!(
            parsed.get("PRETTY_NAME").map(String::as_str),
            Some("Ubuntu 24.04 LTS")
        );
        assert_eq!(
            parsed.get("UNSAFE").map(String::as_str),
            Some("$(touch /tmp/pryn-test)")
        );
    }

    #[test]
    fn parses_only_alsa_card_header_lines() {
        let cards = parse_alsa_cards(
            " 0 [PCH            ]: HDA-Intel - HDA Intel PCH\n                      HDA Intel PCH at 0x... irq 125\n 1 [USB            ]: USB-Audio - USB Audio\n",
        );

        assert_eq!(cards.len(), 2);
        assert!(cards[0].contains("PCH"));
        assert!(cards[1].contains("USB"));
    }
}
