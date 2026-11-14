//! Read-only NetworkManager discovery over the system D-Bus.
//!
//! This module deliberately exposes owned snapshots instead of D-Bus proxies.
//! The diagnostic layer can therefore be tested without a running D-Bus daemon
//! or physical network hardware.

use std::time::Duration;

use zbus::blocking::{Proxy, connection::Builder as ConnectionBuilder, fdo::ObjectManagerProxy};
use zbus::zvariant::OwnedObjectPath;

const DBUS_SERVICE: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";

const NETWORK_MANAGER_SERVICE: &str = "org.freedesktop.NetworkManager";
const NETWORK_MANAGER_PATH: &str = "/org/freedesktop/NetworkManager";
const NETWORK_MANAGER_INTERFACE: &str = "org.freedesktop.NetworkManager";
const NETWORK_MANAGER_DEVICE_INTERFACE: &str = "org.freedesktop.NetworkManager.Device";

const IWD_SERVICE: &str = "net.connman.iwd";
const IWD_DEVICE_INTERFACE: &str = "net.connman.iwd.Device";
const IWD_STATION_INTERFACE: &str = "net.connman.iwd.Station";
const WIFI_DEVICE_TYPE: u32 = 2;
const DBUS_METHOD_TIMEOUT: Duration = Duration::from_secs(3);

/// Which Wi-Fi connection-management implementation could be observed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WifiManagerProbe {
    /// NetworkManager was present and a snapshot was collected.
    NetworkManager(NetworkManagerSnapshot),
    /// iwd was present and its read-only device/station state was collected.
    Iwd(IwdSnapshot),
    /// Neither NetworkManager nor iwd owned its well-known system-bus name.
    NoKnownManager,
    /// PRYN could not connect to the system D-Bus.
    SystemBusUnavailable,
    /// The system bus was reachable, but its service state could not be queried.
    QueryFailed,
}

/// Read-only state returned by standalone iwd.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IwdSnapshot {
    pub devices_query_succeeded: bool,
    pub wifi_devices: Vec<IwdWifiDevice>,
    /// Human-safe descriptions of incomplete property reads.
    pub warnings: Vec<String>,
}

/// State for one iwd-managed wireless station.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IwdWifiDevice {
    pub interface: Option<String>,
    pub powered: Option<bool>,
    /// iwd Device.Mode: station, ap, or ad-hoc.
    pub mode: Option<String>,
    /// iwd Station.State: connected, disconnected, connecting, disconnecting, or roaming.
    pub station_state: Option<String>,
    pub scanning: Option<bool>,
}

/// NetworkManager's view of system-wide connectivity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkManagerConnectivity {
    Unknown,
    None,
    Portal,
    Limited,
    Full,
    Other(u32),
}

impl From<u32> for NetworkManagerConnectivity {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::None,
            2 => Self::Portal,
            3 => Self::Limited,
            4 => Self::Full,
            other => Self::Other(other),
        }
    }
}

/// Read-only state returned by NetworkManager.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NetworkManagerSnapshot {
    pub networking_enabled: Option<bool>,
    pub wireless_enabled: Option<bool>,
    pub wireless_hardware_enabled: Option<bool>,
    pub connectivity_check_enabled: Option<bool>,
    pub connectivity_check_available: Option<bool>,
    pub connectivity: Option<NetworkManagerConnectivity>,
    pub devices_query_succeeded: bool,
    pub wifi_devices: Vec<NetworkManagerWifiDevice>,
    /// Human-safe descriptions of incomplete property reads.
    pub warnings: Vec<String>,
}

/// State for one NetworkManager Wi-Fi device.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkManagerDeviceState {
    Unknown,
    Unmanaged,
    Unavailable,
    Disconnected,
    Preparing,
    Configuring,
    NeedsAuthentication,
    IpConfiguration,
    IpCheck,
    WaitingForSecondary,
    Activated,
    Deactivating,
    Failed,
    Other(u32),
}

impl From<u32> for NetworkManagerDeviceState {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::Unknown,
            10 => Self::Unmanaged,
            20 => Self::Unavailable,
            30 => Self::Disconnected,
            40 => Self::Preparing,
            50 => Self::Configuring,
            60 => Self::NeedsAuthentication,
            70 => Self::IpConfiguration,
            80 => Self::IpCheck,
            90 => Self::WaitingForSecondary,
            100 => Self::Activated,
            110 => Self::Deactivating,
            120 => Self::Failed,
            other => Self::Other(other),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkManagerWifiDevice {
    pub interface: Option<String>,
    pub state: Option<NetworkManagerDeviceState>,
    pub firmware_missing: Option<bool>,
    pub plugin_missing: Option<bool>,
}

/// Collect Wi-Fi manager state without changing system configuration.
///
/// NetworkManager is preferred when present. This also covers systems where
/// NetworkManager uses iwd as its internal Wi-Fi backend, because the public
/// NetworkManager D-Bus service remains the manager-facing API.
pub fn collect_wifi_state() -> WifiManagerProbe {
    let builder = match ConnectionBuilder::system() {
        Ok(builder) => builder.method_timeout(DBUS_METHOD_TIMEOUT),
        Err(error) => {
            tracing::debug!(error = %error, "system D-Bus is unavailable");
            return WifiManagerProbe::SystemBusUnavailable;
        }
    };
    let connection = match builder.build() {
        Ok(connection) => connection,
        Err(error) => {
            tracing::debug!(error = %error, "system D-Bus is unavailable");
            return WifiManagerProbe::SystemBusUnavailable;
        }
    };

    let bus_proxy = match Proxy::new(&connection, DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE) {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::debug!(error = %error, "could not create the D-Bus bus proxy");
            return WifiManagerProbe::QueryFailed;
        }
    };

    let network_manager_present = match name_has_owner(&bus_proxy, NETWORK_MANAGER_SERVICE) {
        Ok(present) => present,
        Err(error) => {
            tracing::debug!(error = %error, "could not check NetworkManager service ownership");
            return WifiManagerProbe::QueryFailed;
        }
    };

    if !network_manager_present {
        return match name_has_owner(&bus_proxy, IWD_SERVICE) {
            Ok(true) => collect_iwd_snapshot(&connection),
            Ok(false) => WifiManagerProbe::NoKnownManager,
            Err(error) => {
                tracing::debug!(error = %error, "could not check iwd service ownership");
                WifiManagerProbe::QueryFailed
            }
        };
    }

    collect_network_manager_snapshot(&connection)
}

fn name_has_owner(proxy: &Proxy<'_>, service_name: &str) -> Result<bool, zbus::Error> {
    proxy.call("NameHasOwner", &(service_name,))
}

fn collect_network_manager_snapshot(connection: &zbus::blocking::Connection) -> WifiManagerProbe {
    let manager_proxy = match Proxy::new(
        connection,
        NETWORK_MANAGER_SERVICE,
        NETWORK_MANAGER_PATH,
        NETWORK_MANAGER_INTERFACE,
    ) {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::debug!(error = %error, "could not create the NetworkManager proxy");
            return WifiManagerProbe::QueryFailed;
        }
    };

    let mut snapshot = NetworkManagerSnapshot::default();
    snapshot.networking_enabled =
        read_bool_property(&manager_proxy, "NetworkingEnabled", &mut snapshot.warnings);
    snapshot.wireless_enabled =
        read_bool_property(&manager_proxy, "WirelessEnabled", &mut snapshot.warnings);
    snapshot.wireless_hardware_enabled = read_bool_property(
        &manager_proxy,
        "WirelessHardwareEnabled",
        &mut snapshot.warnings,
    );
    snapshot.connectivity_check_enabled = read_bool_property(
        &manager_proxy,
        "ConnectivityCheckEnabled",
        &mut snapshot.warnings,
    );
    snapshot.connectivity_check_available = read_bool_property(
        &manager_proxy,
        "ConnectivityCheckAvailable",
        &mut snapshot.warnings,
    );
    snapshot.connectivity =
        read_u32_property(&manager_proxy, "Connectivity", &mut snapshot.warnings)
            .map(NetworkManagerConnectivity::from);

    let device_paths: Vec<OwnedObjectPath> = match manager_proxy.call("GetDevices", &()) {
        Ok(paths) => {
            snapshot.devices_query_succeeded = true;
            paths
        }
        Err(error) => {
            tracing::debug!(error = %error, "NetworkManager GetDevices call failed");
            snapshot
                .warnings
                .push("NetworkManager's device list could not be read.".to_owned());
            Vec::new()
        }
    };

    for path in device_paths {
        let device_proxy = match Proxy::new(
            connection,
            NETWORK_MANAGER_SERVICE,
            path.as_str(),
            NETWORK_MANAGER_DEVICE_INTERFACE,
        ) {
            Ok(proxy) => proxy,
            Err(error) => {
                tracing::debug!(object_path = path.as_str(), error = %error, "could not create a NetworkManager device proxy");
                snapshot
                    .warnings
                    .push("A NetworkManager device could not be inspected.".to_owned());
                continue;
            }
        };

        let device_type = match device_proxy.get_property::<u32>("DeviceType") {
            Ok(value) => value,
            Err(error) => {
                tracing::debug!(object_path = path.as_str(), error = %error, "could not read NetworkManager device type");
                snapshot
                    .warnings
                    .push("A NetworkManager device type could not be read.".to_owned());
                continue;
            }
        };

        if device_type != WIFI_DEVICE_TYPE {
            continue;
        }

        // Read properties into locals before mutating the snapshot. This keeps
        // the mutable warning collection disjoint from the device vector and
        // makes the borrow relationships explicit.
        let interface = read_string_property(&device_proxy, "Interface", &mut snapshot.warnings);
        let state = read_u32_property(&device_proxy, "State", &mut snapshot.warnings)
            .map(NetworkManagerDeviceState::from);
        let firmware_missing =
            read_bool_property(&device_proxy, "FirmwareMissing", &mut snapshot.warnings);
        let plugin_missing =
            read_bool_property(&device_proxy, "NmPluginMissing", &mut snapshot.warnings);

        snapshot.wifi_devices.push(NetworkManagerWifiDevice {
            interface,
            state,
            firmware_missing,
            plugin_missing,
        });
    }

    WifiManagerProbe::NetworkManager(snapshot)
}

fn collect_iwd_snapshot(connection: &zbus::blocking::Connection) -> WifiManagerProbe {
    let object_manager = match ObjectManagerProxy::new(connection, IWD_SERVICE, "/") {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::debug!(error = %error, "could not create the iwd ObjectManager proxy");
            return WifiManagerProbe::QueryFailed;
        }
    };

    let managed_objects = match object_manager.get_managed_objects() {
        Ok(objects) => objects,
        Err(error) => {
            tracing::debug!(error = %error, "iwd GetManagedObjects call failed");
            return WifiManagerProbe::QueryFailed;
        }
    };

    let mut snapshot = IwdSnapshot {
        devices_query_succeeded: true,
        ..IwdSnapshot::default()
    };

    for (path, interfaces) in managed_objects {
        let has_device = interfaces
            .keys()
            .any(|name| name.as_str() == IWD_DEVICE_INTERFACE);
        if !has_device {
            continue;
        }

        let device_proxy =
            match Proxy::new(connection, IWD_SERVICE, path.as_str(), IWD_DEVICE_INTERFACE) {
                Ok(proxy) => proxy,
                Err(error) => {
                    tracing::debug!(error = %error, "could not create an iwd device proxy");
                    snapshot
                        .warnings
                        .push("An iwd wireless device could not be inspected.".to_owned());
                    continue;
                }
            };

        let interface = read_string_property(&device_proxy, "Name", &mut snapshot.warnings);
        let powered = read_bool_property(&device_proxy, "Powered", &mut snapshot.warnings);
        let mode = read_string_property(&device_proxy, "Mode", &mut snapshot.warnings);

        let has_station = interfaces
            .keys()
            .any(|name| name.as_str() == IWD_STATION_INTERFACE);
        let (station_state, scanning) = if has_station {
            match Proxy::new(
                connection,
                IWD_SERVICE,
                path.as_str(),
                IWD_STATION_INTERFACE,
            ) {
                Ok(station_proxy) => (
                    read_string_property(&station_proxy, "State", &mut snapshot.warnings),
                    read_bool_property(&station_proxy, "Scanning", &mut snapshot.warnings),
                ),
                Err(error) => {
                    tracing::debug!(error = %error, "could not create an iwd station proxy");
                    snapshot
                        .warnings
                        .push("An iwd station state could not be inspected.".to_owned());
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        snapshot.wifi_devices.push(IwdWifiDevice {
            interface,
            powered,
            mode,
            station_state,
            scanning,
        });
    }

    snapshot
        .wifi_devices
        .sort_by(|left, right| left.interface.cmp(&right.interface));

    WifiManagerProbe::Iwd(snapshot)
}

fn read_bool_property(
    proxy: &Proxy<'_>,
    property: &str,
    warnings: &mut Vec<String>,
) -> Option<bool> {
    match proxy.get_property::<bool>(property) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::debug!(property, error = %error, "Wi-Fi manager boolean property unavailable");
            warnings.push(format!(
                "Wi-Fi manager property `{property}` could not be read."
            ));
            None
        }
    }
}

fn read_string_property(
    proxy: &Proxy<'_>,
    property: &str,
    warnings: &mut Vec<String>,
) -> Option<String> {
    match proxy.get_property::<String>(property) {
        Ok(value) if !value.trim().is_empty() => Some(value),
        Ok(_) => None,
        Err(error) => {
            tracing::debug!(property, error = %error, "Wi-Fi manager string property unavailable");
            warnings.push(format!(
                "Wi-Fi manager property `{property}` could not be read."
            ));
            None
        }
    }
}

fn read_u32_property(proxy: &Proxy<'_>, property: &str, warnings: &mut Vec<String>) -> Option<u32> {
    match proxy.get_property::<u32>(property) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::debug!(property, error = %error, "Wi-Fi manager integer property unavailable");
            warnings.push(format!(
                "Wi-Fi manager property `{property}` could not be read."
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkManagerConnectivity, NetworkManagerDeviceState, NetworkManagerSnapshot,
        NetworkManagerWifiDevice, WifiManagerProbe,
    };

    #[test]
    fn snapshot_defaults_to_unknown_values_and_no_devices() {
        let snapshot = NetworkManagerSnapshot::default();

        assert_eq!(snapshot.networking_enabled, None);
        assert_eq!(snapshot.wireless_enabled, None);
        assert_eq!(snapshot.wireless_hardware_enabled, None);
        assert_eq!(snapshot.connectivity_check_enabled, None);
        assert_eq!(snapshot.connectivity_check_available, None);
        assert_eq!(snapshot.connectivity, None);
        assert!(!snapshot.devices_query_succeeded);
        assert!(snapshot.wifi_devices.is_empty());
        assert!(snapshot.warnings.is_empty());
    }

    #[test]
    fn snapshot_can_represent_an_active_wifi_device() {
        let probe = WifiManagerProbe::NetworkManager(NetworkManagerSnapshot {
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
        });

        assert!(matches!(
            probe,
            WifiManagerProbe::NetworkManager(snapshot)
                if snapshot.wifi_devices[0].state == Some(NetworkManagerDeviceState::Activated)
                    && snapshot.connectivity == Some(NetworkManagerConnectivity::Full)
        ));
    }

    #[test]
    fn networkmanager_connectivity_values_are_mapped_conservatively() {
        assert_eq!(
            NetworkManagerConnectivity::from(0),
            NetworkManagerConnectivity::Unknown
        );
        assert_eq!(
            NetworkManagerConnectivity::from(2),
            NetworkManagerConnectivity::Portal
        );
        assert_eq!(
            NetworkManagerConnectivity::from(4),
            NetworkManagerConnectivity::Full
        );
        assert_eq!(
            NetworkManagerConnectivity::from(99),
            NetworkManagerConnectivity::Other(99)
        );
    }

    #[test]
    fn networkmanager_device_states_are_mapped_conservatively() {
        assert_eq!(
            NetworkManagerDeviceState::from(30),
            NetworkManagerDeviceState::Disconnected
        );
        assert_eq!(
            NetworkManagerDeviceState::from(100),
            NetworkManagerDeviceState::Activated
        );
        assert_eq!(
            NetworkManagerDeviceState::from(999),
            NetworkManagerDeviceState::Other(999)
        );
    }
}
