//! Read-only host network configuration inspection for Wi-Fi diagnostics.
//!
//! `ip -j` is invoked directly (never through a shell), with bounded execution
//! time and output size. DNS configuration is read from `/etc/resolv.conf`.
//! These checks describe the host's current configuration; they do not perform
//! an internet request or claim that a configured resolver is reachable.

use std::{
    collections::HashSet,
    fs,
    io::Read,
    net::IpAddr,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

const IP_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_IP_OUTPUT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct NetworkConfigSnapshot {
    pub address_query_succeeded: bool,
    pub route_query_succeeded: bool,
    pub dns_config_read: bool,
    pub addresses: Vec<NetworkAddress>,
    pub default_routes: Vec<NetworkRoute>,
    pub dns_servers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkAddress {
    pub interface: String,
    pub family: String,
    pub address: String,
    pub prefix_length: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkRoute {
    pub family: String,
    pub interface: Option<String>,
    pub gateway: Option<String>,
    pub metric: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IpCommandFailure {
    Unavailable,
    TimedOut,
    Failed,
    OutputTooLarge,
    InvalidOutput,
}

impl IpCommandFailure {
    fn summary(self, operation: &str) -> String {
        match self {
            Self::Unavailable => {
                "The `ip` command from iproute2 is not installed; address and route inspection is unavailable.".to_owned()
            }
            Self::TimedOut => format!("`ip {operation}` did not finish within the time limit."),
            Self::Failed => format!("`ip {operation}` could not be completed."),
            Self::OutputTooLarge => format!("`ip {operation}` exceeded PRYN's output limit."),
            Self::InvalidOutput => format!("`ip {operation}` returned JSON that PRYN could not interpret."),
        }
    }
}

/// Collect IP addresses for the supplied wireless interfaces, system default
/// routes, and configured DNS servers without changing network state.
pub fn collect(wifi_interfaces: &[String]) -> NetworkConfigSnapshot {
    let mut snapshot = NetworkConfigSnapshot::default();
    let allowed_interfaces: HashSet<&str> = wifi_interfaces.iter().map(String::as_str).collect();

    match run_ip(&["-j", "address", "show"]) {
        Ok(bytes) => match parse_addresses(&bytes, &allowed_interfaces) {
            Ok(addresses) => {
                snapshot.address_query_succeeded = true;
                snapshot.addresses = addresses;
            }
            Err(failure) => snapshot.warnings.push(failure.summary("-j address show")),
        },
        Err(failure) => snapshot.warnings.push(failure.summary("-j address show")),
    }

    let mut routes = Vec::new();
    let mut route_queries_succeeded = true;
    for (family, arguments, operation) in [
        (
            "IPv4",
            &["-j", "route", "show", "default"][..],
            "-j route show default",
        ),
        (
            "IPv6",
            &["-6", "-j", "route", "show", "default"][..],
            "-6 -j route show default",
        ),
    ] {
        match run_ip(arguments) {
            Ok(bytes) => match parse_default_routes(&bytes, family) {
                Ok(mut parsed) => routes.append(&mut parsed),
                Err(failure) => {
                    route_queries_succeeded = false;
                    snapshot.warnings.push(failure.summary(operation));
                }
            },
            Err(failure) => {
                route_queries_succeeded = false;
                snapshot.warnings.push(failure.summary(operation));
            }
        }
    }
    snapshot.route_query_succeeded = route_queries_succeeded;
    snapshot.default_routes = routes;

    match fs::read_to_string("/etc/resolv.conf") {
        Ok(contents) => {
            snapshot.dns_config_read = true;
            snapshot.dns_servers = parse_dns_servers(&contents);
        }
        Err(error) => snapshot.warnings.push(format!(
            "DNS configuration could not be read from /etc/resolv.conf ({:?}).",
            error.kind()
        )),
    }

    snapshot
}

fn run_ip(arguments: &[&str]) -> Result<Vec<u8>, IpCommandFailure> {
    let mut child = Command::new("ip")
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                IpCommandFailure::Unavailable
            } else {
                IpCommandFailure::Failed
            }
        })?;

    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(IpCommandFailure::Failed);
    };
    let reader = thread::Builder::new()
        .name("pryn-ip-reader".to_owned())
        .spawn(move || drain_limited(stdout))
        .map_err(|_| {
            let _ = child.kill();
            let _ = child.wait();
            IpCommandFailure::Failed
        })?;
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (bytes, too_large, read_ok) =
                    reader.join().map_err(|_| IpCommandFailure::Failed)?;
                if too_large {
                    return Err(IpCommandFailure::OutputTooLarge);
                }
                if !read_ok || !status.success() {
                    return Err(IpCommandFailure::Failed);
                }
                return Ok(bytes);
            }
            Ok(None) if started.elapsed() < IP_COMMAND_TIMEOUT => {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(IpCommandFailure::TimedOut);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(IpCommandFailure::Failed);
            }
        }
    }
}

fn drain_limited(mut reader: impl Read) -> (Vec<u8>, bool, bool) {
    let mut output = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];
    let mut too_large = false;

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return (output, too_large, true),
            Ok(bytes_read) => {
                if output.len().saturating_add(bytes_read) <= MAX_IP_OUTPUT_BYTES {
                    output.extend_from_slice(&buffer[..bytes_read]);
                } else {
                    too_large = true;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return (output, too_large, false),
        }
    }
}

fn parse_addresses(
    bytes: &[u8],
    wifi_interfaces: &HashSet<&str>,
) -> Result<Vec<NetworkAddress>, IpCommandFailure> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| IpCommandFailure::InvalidOutput)?;
    let interfaces = root.as_array().ok_or(IpCommandFailure::InvalidOutput)?;
    let mut addresses = Vec::new();

    for interface in interfaces {
        let Some(name) = interface.get("ifname").and_then(Value::as_str) else {
            continue;
        };
        if !wifi_interfaces.contains(name) {
            continue;
        }
        let Some(entries) = interface.get("addr_info").and_then(Value::as_array) else {
            continue;
        };

        for entry in entries {
            let Some(family) = entry.get("family").and_then(Value::as_str) else {
                continue;
            };
            if family != "inet" && family != "inet6" {
                continue;
            }
            if entry.get("scope").and_then(Value::as_str) != Some("global") {
                continue;
            }
            let Some(address) = entry.get("local").and_then(Value::as_str) else {
                continue;
            };
            if is_non_routable_host_address(address) {
                continue;
            }

            addresses.push(NetworkAddress {
                interface: name.to_owned(),
                family: if family == "inet" { "IPv4" } else { "IPv6" }.to_owned(),
                address: address.to_owned(),
                prefix_length: entry.get("prefixlen").and_then(Value::as_u64),
            });
        }
    }

    Ok(addresses)
}

fn parse_default_routes(bytes: &[u8], family: &str) -> Result<Vec<NetworkRoute>, IpCommandFailure> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| IpCommandFailure::InvalidOutput)?;
    let routes = root.as_array().ok_or(IpCommandFailure::InvalidOutput)?;

    Ok(routes
        .iter()
        .filter(|route| {
            route.get("dst").and_then(Value::as_str) == Some("default")
                || route.get("dst").is_none()
        })
        .map(|route| NetworkRoute {
            family: family.to_owned(),
            interface: route.get("dev").and_then(Value::as_str).map(str::to_owned),
            gateway: route
                .get("gateway")
                .and_then(Value::as_str)
                .map(str::to_owned),
            metric: route.get("metric").and_then(Value::as_u64),
        })
        .collect())
}

fn parse_dns_servers(contents: &str) -> Vec<String> {
    let mut servers = Vec::new();
    for line in contents.lines() {
        let uncommented = line.split('#').next().unwrap_or_default();
        let mut fields = uncommented.split_whitespace();
        if fields.next() != Some("nameserver") {
            continue;
        }
        let Some(address) = fields.next() else {
            continue;
        };
        if address.parse::<IpAddr>().is_ok()
            && !servers
                .iter()
                .any(|known: &String| known.as_str() == address)
        {
            servers.push(address.to_owned());
        }
    }
    servers
}

fn is_non_routable_host_address(value: &str) -> bool {
    match value.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => {
            address.is_loopback()
                || address.is_unspecified()
                || address.is_multicast()
                || (address.octets()[0] == 169 && address.octets()[1] == 254)
        }
        Ok(IpAddr::V6(address)) => {
            address.is_loopback()
                || address.is_unspecified()
                || address.is_multicast()
                || address.is_unicast_link_local()
        }
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{parse_addresses, parse_default_routes, parse_dns_servers};
    use std::collections::HashSet;

    #[test]
    fn parses_only_global_addresses_for_wireless_interfaces() {
        let json = br#"[
            {"ifname":"wlp2s0","addr_info":[
                {"family":"inet","local":"192.0.2.10","prefixlen":24,"scope":"global"},
                {"family":"inet6","local":"fe80::1","prefixlen":64,"scope":"link"},
                {"family":"inet6","local":"2001:db8::2","prefixlen":64},
                {"family":"inet6","local":"ff02::1","prefixlen":16,"scope":"global"}
            ]},
            {"ifname":"eth0","addr_info":[
                {"family":"inet","local":"192.0.2.20","prefixlen":24,"scope":"global"}
            ]}
        ]"#;
        let interfaces = HashSet::from(["wlp2s0"]);
        let addresses = parse_addresses(json, &interfaces).expect("valid address JSON");
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].interface, "wlp2s0");
        assert_eq!(addresses[0].address, "192.0.2.10");
        assert_eq!(addresses[0].prefix_length, Some(24));
    }

    #[test]
    fn parses_ipv4_and_ipv6_default_routes() {
        let ipv4 = br#"[{"dst":"default","gateway":"192.0.2.1","dev":"wlp2s0","metric":600}]"#;
        let ipv6 = br#"[{"dst":"default","gateway":"fe80::1","dev":"wlp2s0","metric":600}]"#;
        let ipv4_routes = parse_default_routes(ipv4, "IPv4").expect("valid IPv4 route JSON");
        let ipv6_routes = parse_default_routes(ipv6, "IPv6").expect("valid IPv6 route JSON");

        assert_eq!(ipv4_routes.len(), 1);
        assert_eq!(ipv4_routes[0].interface.as_deref(), Some("wlp2s0"));
        assert_eq!(ipv4_routes[0].gateway.as_deref(), Some("192.0.2.1"));
        assert_eq!(ipv6_routes.len(), 1);
        assert_eq!(ipv6_routes[0].gateway.as_deref(), Some("fe80::1"));
    }

    #[test]
    fn parses_and_deduplicates_dns_nameservers() {
        let dns = parse_dns_servers(
            "# comment\nnameserver 1.1.1.1\nnameserver 2606:4700:4700::1111\nnameserver 1.1.1.1\n",
        );
        assert_eq!(dns, vec!["1.1.1.1", "2606:4700:4700::1111"]);
    }
}
