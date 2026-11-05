# Support matrix (early development)

PRYN targets Linux. Core hardware discovery uses Linux kernel interfaces. Service-level checks are capability-based and depend on the service being present and reachable.

| Environment | Build target | Wi-Fi support in this version | Bluetooth / Audio |
|---|---|---|---|
| Arch Linux x86_64 | Development target | sysfs/rfkill discovery; NetworkManager D-Bus read-only checks, including its last system-wide connectivity result when configured and enabled; iwd detection only | Hardware discovery and rfkill; service/routing checks not implemented |
| Ubuntu x86_64 | Development target | sysfs/rfkill discovery; NetworkManager D-Bus read-only checks, including its last system-wide connectivity result when configured and enabled; iwd detection only | Hardware discovery and rfkill; service/routing checks not implemented |
| Other Linux distributions | Not yet validated | Best effort where Linux interfaces and NetworkManager D-Bus are available | Best effort hardware discovery only |
| Non-systemd Linux | Not yet validated | NetworkManager D-Bus checks can work independently of systemd; other paths unverified | Best effort hardware discovery only |
| aarch64 Linux | Not yet validated | Not yet claimed | Not yet claimed |

A successful build on a distribution is not proof that its service configuration is supported. Update this matrix only after real testing on that environment. NetworkManager is not assumed to be the only Wi-Fi manager; iwd is detected but direct inspection is still pending.
