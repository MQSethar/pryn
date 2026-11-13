# Support matrix (early development)

PRYN targets Linux. Core hardware discovery uses Linux kernel interfaces. Service-level checks are capability-based and depend on the service and optional inspection tools being present in the current user session.

| Environment | Build target | Wi-Fi support | Bluetooth support | Audio support |
|---|---|---|---|---|
| Arch Linux x86_64 | Primary development target | sysfs/rfkill, NetworkManager D-Bus, and optional iproute2 address/route inspection plus `/etc/resolv.conf` DNS configuration | sysfs/rfkill plus BlueZ system D-Bus adapter, pairing, and connection-state inspection | ALSA discovery; PulseAudio-compatible protocol inspection; native graph inspection when `pw-dump` is installed |
| Ubuntu x86_64 | CI build target; runtime validation required | sysfs/rfkill, NetworkManager D-Bus, and optional iproute2 address/route inspection plus `/etc/resolv.conf` DNS configuration | sysfs/rfkill plus BlueZ system D-Bus adapter, pairing, and connection-state inspection | ALSA discovery; PulseAudio-compatible protocol inspection; native PipeWire graph inspection when available |
| Other Linux distributions | Not yet validated | Best effort where Linux interfaces, NetworkManager D-Bus, and optional iproute2 are available | sysfs/rfkill plus best-effort BlueZ system D-Bus inspection | Best-effort ALSA; PulseAudio-compatible socket and optional native PipeWire `pw-dump` providers |
| Non-systemd Linux | Not yet validated | NetworkManager D-Bus can work independently of systemd; other paths unverified | sysfs/rfkill plus best-effort BlueZ system D-Bus inspection | PulseAudio-compatible socket provider and PipeWire `pw-dump` provider do not require systemd, but runtime support is not yet validated |
| aarch64 Linux | Not yet validated | Not yet claimed | Not yet claimed | Not yet claimed |

A successful build on a distribution is not proof that its service configuration is supported. Update this matrix only after real testing on that environment. Native PulseAudio and PipeWire are separate providers; PipeWire's PulseAudio-compatible service is also supported through the common PulseAudio client protocol. Direct iwd connection-state inspection and active internet reachability tests are not supported in the first MVP. The `ip` command is optional: when it is absent, IP-address and default-route results remain unknown rather than being reported as successful.
