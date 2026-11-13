//! Read-only Bluetooth inventory through BlueZ's system D-Bus API.
//!
//! BlueZ exposes adapters and devices through ObjectManager. PRYN reads one
//! managed-object snapshot so that adapter and device properties are observed
//! together, without starting discovery, pairing, connecting, or changing any
//! radio settings. Bluetooth addresses are deliberately not copied into the
//! diagnostic model or report.

use std::collections::HashMap;
use std::time::Duration;

use zbus::blocking::{Proxy, connection::Builder as ConnectionBuilder};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const DBUS_SERVICE: &str = "org.freedesktop.DBus";
const DBUS_PATH: &str = "/org/freedesktop/DBus";
const DBUS_INTERFACE: &str = "org.freedesktop.DBus";
const BLUEZ_SERVICE: &str = "org.bluez";
const BLUEZ_ROOT: &str = "/";
const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";
const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
const DEVICE_INTERFACE: &str = "org.bluez.Device1";
const DBUS_METHOD_TIMEOUT: Duration = Duration::from_secs(3);

type PropertyMap = HashMap<String, OwnedValue>;
type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, PropertyMap>>;

/// Result of checking whether BlueZ can be inspected on the system bus.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BluezProbe {
    Available(BluezSnapshot),
    NotAvailable,
    SystemBusUnavailable,
    QueryFailed,
}

/// One read-only snapshot of BlueZ's managed Bluetooth objects.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BluezSnapshot {
    pub adapters: Vec<BluezAdapter>,
    pub devices: Vec<BluezDevice>,
    pub warnings: Vec<String>,
}

/// Bluetooth controller state reported by BlueZ.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluezAdapter {
    /// Human-readable controller name, or its kernel controller name.
    pub name: String,
    pub powered: Option<bool>,
    pub discovering: Option<bool>,
    pub discoverable: Option<bool>,
    pub pairable: Option<bool>,
}

/// Device state reported by BlueZ. Addresses and D-Bus object paths are not
/// retained, so reports do not expose Bluetooth hardware identifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BluezDevice {
    pub name: Option<String>,
    pub adapter: Option<String>,
    pub paired: Option<bool>,
    pub trusted: Option<bool>,
    pub connected: Option<bool>,
    pub blocked: Option<bool>,
    pub services_resolved: Option<bool>,
}

/// Inspect BlueZ without initiating discovery or changing any device state.
pub fn collect() -> BluezProbe {
    let builder = match ConnectionBuilder::system() {
        Ok(builder) => builder.method_timeout(DBUS_METHOD_TIMEOUT),
        Err(error) => {
            tracing::debug!(error = %error, "system D-Bus is unavailable for BlueZ inspection");
            return BluezProbe::SystemBusUnavailable;
        }
    };
    let connection = match builder.build() {
        Ok(connection) => connection,
        Err(error) => {
            tracing::debug!(error = %error, "could not connect to the system D-Bus for BlueZ inspection");
            return BluezProbe::SystemBusUnavailable;
        }
    };

    let bus_proxy = match Proxy::new(&connection, DBUS_SERVICE, DBUS_PATH, DBUS_INTERFACE) {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::debug!(error = %error, "could not create the system D-Bus proxy");
            return BluezProbe::QueryFailed;
        }
    };

    let bluez_present: bool = match bus_proxy.call("NameHasOwner", &(BLUEZ_SERVICE,)) {
        Ok(present) => present,
        Err(error) => {
            tracing::debug!(error = %error, "could not check BlueZ service ownership");
            return BluezProbe::QueryFailed;
        }
    };
    if !bluez_present {
        return BluezProbe::NotAvailable;
    }

    let object_manager = match Proxy::new(
        &connection,
        BLUEZ_SERVICE,
        BLUEZ_ROOT,
        OBJECT_MANAGER_INTERFACE,
    ) {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::debug!(error = %error, "could not create the BlueZ ObjectManager proxy");
            return BluezProbe::QueryFailed;
        }
    };

    let objects: ManagedObjects = match object_manager.call("GetManagedObjects", &()) {
        Ok(objects) => objects,
        Err(error) => {
            tracing::debug!(error = %error, "BlueZ GetManagedObjects failed");
            return BluezProbe::QueryFailed;
        }
    };

    BluezProbe::Available(snapshot_from_managed_objects(&objects))
}

fn snapshot_from_managed_objects(objects: &ManagedObjects) -> BluezSnapshot {
    let mut snapshot = BluezSnapshot::default();

    for (object_path, interfaces) in objects {
        if let Some(properties) = interfaces.get(ADAPTER_INTERFACE) {
            let controller_name = object_path
                .as_str()
                .rsplit('/')
                .next()
                .filter(|name| !name.is_empty())
                .unwrap_or("Bluetooth adapter");
            let name = property_string(properties, "Alias")
                .filter(|name| !name.trim().is_empty() && !looks_like_bluetooth_address(name))
                .or_else(|| {
                    property_string(properties, "Name")
                        .filter(|name| !looks_like_bluetooth_address(name))
                })
                .unwrap_or_else(|| controller_name.to_owned());

            snapshot.adapters.push(BluezAdapter {
                name,
                powered: property_bool(properties, "Powered"),
                discovering: property_bool(properties, "Discovering"),
                discoverable: property_bool(properties, "Discoverable"),
                pairable: property_bool(properties, "Pairable"),
            });

            if property_bool(properties, "Powered").is_none() {
                snapshot.warnings.push(format!(
                    "BlueZ did not expose the power state for {controller_name}."
                ));
            }
        }

        if let Some(properties) = interfaces.get(DEVICE_INTERFACE) {
            let adapter_path = property_object_path(properties, "Adapter");
            let adapter = adapter_path
                .as_deref()
                .and_then(|path| path.rsplit('/').next())
                .filter(|name| !name.is_empty())
                .map(str::to_owned);
            let name = property_string(properties, "Alias")
                .filter(|name| !name.trim().is_empty() && !looks_like_bluetooth_address(name))
                .or_else(|| {
                    property_string(properties, "Name")
                        .filter(|name| !looks_like_bluetooth_address(name))
                });

            snapshot.devices.push(BluezDevice {
                name,
                adapter,
                paired: property_bool(properties, "Paired"),
                trusted: property_bool(properties, "Trusted"),
                connected: property_bool(properties, "Connected"),
                blocked: property_bool(properties, "Blocked"),
                services_resolved: property_bool(properties, "ServicesResolved"),
            });
        }
    }

    snapshot
        .adapters
        .sort_by(|left, right| left.name.cmp(&right.name));
    snapshot.devices.sort_by(|left, right| {
        left.name
            .as_deref()
            .unwrap_or_default()
            .cmp(right.name.as_deref().unwrap_or_default())
    });
    snapshot.warnings.sort();
    snapshot
}

fn property_bool(properties: &PropertyMap, key: &str) -> Option<bool> {
    properties
        .get(key)
        .and_then(|value| bool::try_from(value.clone()).ok())
}

fn property_string(properties: &PropertyMap, key: &str) -> Option<String> {
    properties
        .get(key)
        .and_then(|value| String::try_from(value.clone()).ok())
}

fn looks_like_bluetooth_address(value: &str) -> bool {
    let separator = if value.contains(':') {
        ':'
    } else if value.contains('_') {
        '_'
    } else {
        return false;
    };
    let octets = value.split(separator).collect::<Vec<_>>();
    octets.len() == 6
        && octets
            .iter()
            .all(|octet| octet.len() == 2 && octet.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn property_object_path(properties: &PropertyMap, key: &str) -> Option<String> {
    properties.get(key).and_then(|value| {
        OwnedObjectPath::try_from(value.clone())
            .ok()
            .map(|path| path.as_str().to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ADAPTER_INTERFACE, DEVICE_INTERFACE, ManagedObjects, OwnedObjectPath, OwnedValue,
        snapshot_from_managed_objects,
    };
    use std::collections::HashMap;
    use zbus::zvariant::{ObjectPath, Str};

    fn object_path(value: &str) -> OwnedObjectPath {
        OwnedObjectPath::try_from(value).expect("valid D-Bus object path")
    }

    fn string_value(value: &str) -> OwnedValue {
        OwnedValue::from(Str::from(value.to_owned()))
    }

    fn object_path_value(value: &str) -> OwnedValue {
        OwnedValue::from(ObjectPath::try_from(value.to_owned()).expect("valid D-Bus object path"))
    }

    #[test]
    fn extracts_adapter_and_device_state_without_retaining_addresses() {
        let mut adapter_properties = HashMap::new();
        adapter_properties.insert("Alias".to_owned(), string_value("Laptop Bluetooth"));
        adapter_properties.insert("Powered".to_owned(), OwnedValue::from(true));
        adapter_properties.insert("Discovering".to_owned(), OwnedValue::from(false));
        adapter_properties.insert("Discoverable".to_owned(), OwnedValue::from(false));
        adapter_properties.insert("Pairable".to_owned(), OwnedValue::from(true));

        let mut device_properties = HashMap::new();
        device_properties.insert("Alias".to_owned(), string_value("Wireless Headphones"));
        device_properties.insert("Adapter".to_owned(), object_path_value("/org/bluez/hci0"));
        device_properties.insert("Paired".to_owned(), OwnedValue::from(true));
        device_properties.insert("Trusted".to_owned(), OwnedValue::from(true));
        device_properties.insert("Connected".to_owned(), OwnedValue::from(true));
        device_properties.insert("Blocked".to_owned(), OwnedValue::from(false));
        device_properties.insert("ServicesResolved".to_owned(), OwnedValue::from(true));
        // The Device1 Address property is deliberately not copied into the snapshot.
        device_properties.insert("Address".to_owned(), string_value("AA:BB:CC:DD:EE:FF"));

        let mut adapter_interfaces = HashMap::new();
        adapter_interfaces.insert(ADAPTER_INTERFACE.to_owned(), adapter_properties);
        let mut device_interfaces = HashMap::new();
        device_interfaces.insert(DEVICE_INTERFACE.to_owned(), device_properties);

        let objects: ManagedObjects = HashMap::from([
            (object_path("/org/bluez/hci0"), adapter_interfaces),
            (
                object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
                device_interfaces,
            ),
        ]);

        let snapshot = snapshot_from_managed_objects(&objects);
        assert_eq!(snapshot.adapters.len(), 1);
        assert_eq!(snapshot.adapters[0].name, "Laptop Bluetooth");
        assert_eq!(snapshot.adapters[0].powered, Some(true));
        assert_eq!(snapshot.devices.len(), 1);
        assert_eq!(
            snapshot.devices[0].name.as_deref(),
            Some("Wireless Headphones")
        );
        assert_eq!(snapshot.devices[0].adapter.as_deref(), Some("hci0"));
        assert_eq!(snapshot.devices[0].paired, Some(true));
        assert_eq!(snapshot.devices[0].connected, Some(true));
        assert_eq!(snapshot.devices[0].trusted, Some(true));
        assert_eq!(snapshot.devices[0].services_resolved, Some(true));
    }

    #[test]
    fn address_like_device_alias_is_not_used_as_a_display_name() {
        let mut device_properties = HashMap::new();
        device_properties.insert("Alias".to_owned(), string_value("AA:BB:CC:DD:EE:FF"));
        device_properties.insert("Adapter".to_owned(), object_path_value("/org/bluez/hci0"));
        let mut device_interfaces = HashMap::new();
        device_interfaces.insert(DEVICE_INTERFACE.to_owned(), device_properties);
        let objects: ManagedObjects = HashMap::from([(
            object_path("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF"),
            device_interfaces,
        )]);

        let snapshot = snapshot_from_managed_objects(&objects);
        assert_eq!(snapshot.devices.len(), 1);
        assert_eq!(snapshot.devices[0].name, None);
    }

    #[test]
    fn missing_optional_properties_remain_unknown() {
        let mut adapter_interfaces = HashMap::new();
        adapter_interfaces.insert(ADAPTER_INTERFACE.to_owned(), HashMap::new());
        let objects: ManagedObjects =
            HashMap::from([(object_path("/org/bluez/hci0"), adapter_interfaces)]);

        let snapshot = snapshot_from_managed_objects(&objects);
        assert_eq!(snapshot.adapters.len(), 1);
        assert_eq!(snapshot.adapters[0].powered, None);
        assert_eq!(snapshot.warnings.len(), 1);
    }
}
