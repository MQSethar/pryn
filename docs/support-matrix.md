# Support matrix (early development)

PRYN targets Linux. Core hardware discovery uses Linux kernel interfaces. Service-level checks are capability-based and depend on the service and optional inspection tools being present in the current user session.

| Environment | Build target | Wi-Fi support | Bluetooth support | Audio support |
|---|---|---|---|---|
| Arch Linux x86_64 | Development target | sysfs/rfkill and NetworkManager D-Bus; iwd detection only | Controller discovery and rfkill; BlueZ service/device checks not yet implemented | ALSA discovery; PipeWire `pipewire-pulse` protocol inspection; native graph inspection when `pw-dump` is installed |
| Ubuntu x86_64 | Development target | sysfs/rfkill and NetworkManager D-Bus; iwd detection only | Controller discovery and rfkill; BlueZ service/device checks not yet implemented | ALSA discovery; native PulseAudio protocol inspection; native PipeWire graph inspection only if a PipeWire daemon and `pw-dump` are available |
| Other Linux distributions | Not yet validated | Best effort where Linux interfaces and NetworkManager D-Bus are available | Best effort hardware discovery | Best-effort ALSA; PulseAudio-compatible socket and optional native PipeWire `pw-dump` providers |
| Non-systemd Linux | Not yet validated | NetworkManager D-Bus can work independently of systemd; other paths unverified | Best effort hardware discovery | PulseAudio-compatible socket provider and PipeWire `pw-dump` provider do not require systemd, but runtime support is not yet validated |
| aarch64 Linux | Not yet validated | Not yet claimed | Not yet claimed | Not yet claimed |

A successful build on a distribution is not proof that its service configuration is supported. Update this matrix only after real testing on that environment. Native PulseAudio and PipeWire are separate providers; PipeWire's PulseAudio-compatible service is also supported through the common PulseAudio client protocol.
