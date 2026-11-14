# Audio diagnostics v1

Audio diagnostics combine ALSA hardware inventory with two independent user-session service providers.

## Providers

### PulseAudio client protocol

PRYN connects to the current user's local PulseAudio-compatible Unix socket. Native PulseAudio and PipeWire's `pipewire-pulse` compatibility service implement this protocol. The collector uses a pure-Rust protocol implementation and returns owned snapshots. It does not call `pactl`, `wpctl`, or a shell.

Collected data includes server identity/version, default output/input names, playback sinks, non-monitor capture sources, mute state, per-channel volume, sample format, sample rate, channel count, selectable ports and availability, and card profiles.

The authentication cookie is passed only during the protocol handshake. It is not logged, serialized, or included in diagnostic evidence. The handshake has bounded socket timeouts; all four introspection queries share a five-second deadline.

### Native PipeWire graph

When `pw-dump` is installed, PRYN invokes it directly with `--no-colors` and parses the JSON representation of the native PipeWire graph. It inspects Core, Node, Device and default metadata objects. Application Client and Stream objects are intentionally discarded to avoid collecting application names or media metadata.

The `pw-dump` process is limited to three seconds and 8 MiB of output. The native provider adds graph visibility and can supply basic endpoint/default-device observations when the PulseAudio-compatible socket is absent. PipeWire's `device.api` is retained as a device-API value, not mislabeled as a kernel driver. Native graph node presence does not establish physical connectivity or audible playback.

### ALSA hardware

The existing sysfs collector independently discovers `/sys/class/sound` cards and driver symlinks. Hardware discovery continues to work when no user-session audio server is running.

### Bluetooth audio

Audio card snapshots identify Bluetooth audio cards through PipeWire's `device.api=bluez5` property or the corresponding BlueZ audio card/driver naming in PulseAudio-compatible providers. PRYN reports the active Bluetooth audio profile separately from BlueZ pairing/connection state. A paired Bluetooth device does not automatically imply that an A2DP playback or HFP/HSP capture profile is active. An `off` profile is not treated as a generic audio-card failure because BlueZ cards can remain visible after a headset disconnects; the profile check reports that state as not applicable when every Bluetooth audio card is off. If a provider reports a non-`off` active profile as unavailable, or cannot confirm the active profile's availability, PRYN reports the profile check as unknown rather than claiming that the profile is healthy or the hardware is broken. Bluetooth card labels, endpoint identifiers, and MAC-like device labels are sanitized in diagnostic output.

## Diagnostic interpretation

- `SUSPENDED` is a normal idle state. It is not a failure by itself.
- A muted default output is a warning, but may be intentional.
- A zero default output level is reported separately from mute state.
- Input mute is surfaced as an informational privacy state, not an instruction to unmute.
- PulseAudio channel volume is normalized against `PA_VOLUME_NORM` so the report matches the server's user-facing level (for example, 5% remains 5%). PRYN does not apply PulseAudio's cubic-to-linear acoustic-gain conversion to sink/source levels; PulseAudio documents that such conversions should not be used unless the device advertises decibel-volume support.
- Volume above 100% is configured amplification and may cause distortion; PRYN cannot prove clipping from configuration alone.
- A channel configured at zero while other channels are active is a possible balance issue, not proof of a damaged speaker/microphone.
- Monitor sources capture playback output. They are not counted as microphone inputs.
- Non-monitor sources may still be virtual; PRYN does not claim they are physical microphones without stronger evidence.
- Port availability and active profiles are reported only when the provider exposes them. Absence of port data is not a physical disconnection. When the provider explicitly says a selected port is unknown, the related check remains unknown rather than inferring a successful connection.
- `pw-dump` graph data and PulseAudio-compatible endpoint data are separate provider observations. When both are available, PulseAudio-compatible details are used for richer endpoint checks and the native graph is reported independently.
- Bluetooth pairing/connection state is collected separately from audio profile state. A connected Bluetooth device is not proof that an audio profile is active or that audio is flowing. PRYN does not infer a Bluetooth audio failure solely from an `off` profile because it cannot safely correlate a generic BlueZ device object with an audio card without exposing stable hardware identifiers.

## Explicitly not tested by `pryn check audio`

PRYN does not play samples, record microphone audio, enumerate active application streams, inspect per-application volume, or change profiles/default devices/volume/mute state. Those operations require an explicit future `pryn test audio` workflow and clear user consent.
