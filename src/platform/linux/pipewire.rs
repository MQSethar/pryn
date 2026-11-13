//! Read-only native PipeWire graph inspection through `pw-dump` JSON.
//!
//! The PipeWire CLI is used as a process adapter, not through a shell. Its
//! documented JSON output exposes native Core, Node, Device and Metadata
//! objects. The collector retains only audio-device information and default
//! routing metadata; it discards client/application stream objects.

use std::{
    io::Read,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

use super::audio_server::{
    AudioAvailability, AudioCard, AudioEndpoint, AudioEndpointState, AudioProfile, AudioSampleSpec,
    AudioServerKind, AudioServerSnapshot, AudioSource, AudioVolume,
};

const PW_DUMP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_DUMP_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct PipeWireSnapshot {
    pub core_name: Option<String>,
    pub core_version: Option<String>,
    pub default_sink_name: Option<String>,
    pub default_source_name: Option<String>,
    pub sinks: Vec<AudioEndpoint>,
    pub sources: Vec<AudioSource>,
    pub cards: Vec<AudioCard>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PipeWireFailure {
    ToolUnavailable,
    ServerUnavailable,
    TimedOut,
    InvalidOutput,
    OutputTooLarge,
    Failed,
}

impl PipeWireFailure {
    pub const fn user_summary(self) -> &'static str {
        match self {
            Self::ToolUnavailable => {
                "The native PipeWire inspection tool `pw-dump` is not installed."
            }
            Self::ServerUnavailable => {
                "The native PipeWire daemon is not available in this user session."
            }
            Self::TimedOut => "The native PipeWire graph did not respond within the time limit.",
            Self::InvalidOutput => {
                "The native PipeWire tool returned data that PRYN could not interpret."
            }
            Self::OutputTooLarge => {
                "The native PipeWire graph exceeded PRYN's inspection size limit."
            }
            Self::Failed => "PRYN could not complete native PipeWire inspection.",
        }
    }
}

impl PipeWireSnapshot {
    /// Convert native PipeWire observations into the shared endpoint model.
    /// Native port availability is not inferred from graph-node existence.
    pub fn as_audio_server_snapshot(&self) -> AudioServerSnapshot {
        AudioServerSnapshot {
            server_kind: AudioServerKind::PipeWireNative,
            server_name: Some("PipeWire".to_owned()),
            server_version: self.core_version.clone(),
            default_sink_name: self.default_sink_name.clone(),
            default_source_name: self.default_source_name.clone(),
            server_sample_spec: AudioSampleSpec::default(),
            sinks: Some(self.sinks.clone()),
            sources: Some(self.sources.clone()),
            cards: Some(self.cards.clone()),
            warnings: self.warnings.clone(),
        }
    }
}

/// Inspect the current user's native PipeWire graph with a bounded process
/// adapter. No shell is used and no graph state is changed.
pub fn collect() -> Result<PipeWireSnapshot, PipeWireFailure> {
    let bytes = run_pw_dump()?;
    parse_dump(&bytes)
}

fn run_pw_dump() -> Result<Vec<u8>, PipeWireFailure> {
    let mut child = Command::new("pw-dump")
        .arg("--no-colors")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                PipeWireFailure::ToolUnavailable
            } else {
                PipeWireFailure::Failed
            }
        })?;

    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(PipeWireFailure::Failed);
    };
    let reader = match thread::Builder::new()
        .name("pryn-pw-dump-reader".to_owned())
        .spawn(move || drain_limited(stdout))
    {
        Ok(reader) => reader,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(PipeWireFailure::Failed);
        }
    };
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let (bytes, too_large, read_ok) =
                    reader.join().map_err(|_| PipeWireFailure::Failed)?;
                if too_large {
                    return Err(PipeWireFailure::OutputTooLarge);
                }
                if !read_ok {
                    return Err(PipeWireFailure::Failed);
                }
                if !status.success() {
                    return Err(PipeWireFailure::ServerUnavailable);
                }
                return Ok(bytes);
            }
            Ok(None) if started.elapsed() < PW_DUMP_TIMEOUT => {
                thread::sleep(Duration::from_millis(10));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(PipeWireFailure::TimedOut);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                return Err(PipeWireFailure::Failed);
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
                if output.len().saturating_add(bytes_read) <= MAX_DUMP_BYTES {
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

fn parse_dump(bytes: &[u8]) -> Result<PipeWireSnapshot, PipeWireFailure> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| PipeWireFailure::InvalidOutput)?;
    let objects = root.as_array().ok_or(PipeWireFailure::InvalidOutput)?;
    let mut snapshot = PipeWireSnapshot::default();
    let mut has_core = false;

    for object in objects {
        let Some(object_type) = object.get("type").and_then(Value::as_str) else {
            continue;
        };
        let info = object.get("info").unwrap_or(object);
        let props = info.get("props").unwrap_or(&Value::Null);

        if object_type.ends_with(":Core") {
            has_core = true;
            snapshot.core_name =
                string_prop(props, "core.name").or_else(|| string_prop(props, "name"));
            snapshot.core_version = info.get("version").and_then(value_string);
            continue;
        }

        if object_type.ends_with(":Node") {
            parse_node(info, props, &mut snapshot);
            continue;
        }

        if object_type.ends_with(":Device") {
            if let Some(card) = parse_device(info, props) {
                snapshot.cards.push(card);
            }
            continue;
        }

        if object_type.ends_with(":Metadata") {
            parse_metadata(info, object, &mut snapshot);
        }
    }

    if !has_core {
        return Err(PipeWireFailure::ServerUnavailable);
    }

    snapshot
        .sinks
        .sort_by(|left, right| left.name.cmp(&right.name));
    snapshot
        .sources
        .sort_by(|left, right| left.endpoint.name.cmp(&right.endpoint.name));
    snapshot
        .cards
        .sort_by(|left, right| left.name.cmp(&right.name));
    Ok(snapshot)
}

fn parse_node(info: &Value, props: &Value, snapshot: &mut PipeWireSnapshot) {
    let Some(media_class) = string_prop(props, "media.class") else {
        return;
    };
    let Some(name) = string_prop(props, "node.name") else {
        return;
    };
    let description =
        string_prop(props, "node.description").or_else(|| string_prop(props, "node.nick"));
    let is_source = media_class == "Audio/Source" || media_class.starts_with("Audio/Source/");
    let is_monitor = is_source
        && (media_class.contains("Monitor")
            || name.to_ascii_lowercase().ends_with(".monitor")
            || props.get("stream.capture.sink").and_then(Value::as_bool) == Some(true));
    let endpoint = parse_endpoint(info, props, name, description);

    if media_class == "Audio/Sink" || media_class.starts_with("Audio/Sink/") {
        snapshot.sinks.push(endpoint);
    } else if media_class == "Audio/Duplex" {
        snapshot.sinks.push(endpoint.clone());
        snapshot.sources.push(AudioSource {
            is_monitor: false,
            endpoint,
        });
    } else if is_source {
        snapshot.sources.push(AudioSource {
            endpoint,
            is_monitor,
        });
    }
}

fn parse_endpoint(
    info: &Value,
    props: &Value,
    name: String,
    description: Option<String>,
) -> AudioEndpoint {
    let params = info.get("params");
    let node_props = first_object(params.and_then(|params| params.get("Props")));
    let state = match info.get("state").and_then(Value::as_str) {
        Some("running") => AudioEndpointState::Running,
        Some("idle") => AudioEndpointState::Idle,
        Some("suspended") => AudioEndpointState::Suspended,
        _ => AudioEndpointState::Unknown,
    };
    let sample_spec = AudioSampleSpec {
        format: string_prop(props, "audio.format"),
        sample_rate_hz: number_prop_u32(props, "audio.rate"),
        channels: number_prop_u8(props, "audio.channels"),
    };

    AudioEndpoint {
        name,
        description,
        state,
        muted: node_props
            .and_then(|value| value.get("mute"))
            .and_then(Value::as_bool),
        volume: parse_volume(node_props),
        sample_spec,
        ports: Vec::new(),
        active_port: None,
    }
}

fn parse_volume(props: Option<&Value>) -> AudioVolume {
    let channels = props
        .and_then(|props| props.get("channelVolumes"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_f64)
                .map(|value| value as f32)
                .filter(|value| value.is_finite())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let values = if channels.is_empty() {
        props
            .and_then(|props| props.get("volume"))
            .and_then(Value::as_f64)
            .map(|value| vec![value as f32])
            .unwrap_or_default()
    } else {
        channels
    };

    if values.is_empty() {
        return AudioVolume::default();
    }

    let minimum = values.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let average = values.iter().sum::<f32>() / values.len() as f32;
    AudioVolume {
        minimum: Some(minimum),
        maximum: Some(maximum),
        average: Some(average),
    }
}

fn parse_device(info: &Value, props: &Value) -> Option<AudioCard> {
    if string_prop(props, "media.class").as_deref() != Some("Audio/Device") {
        return None;
    }
    let name = string_prop(props, "device.name").or_else(|| string_prop(props, "object.path"))?;
    let device_api = string_prop(props, "device.api");
    let params = info.get("params");
    let available_profiles = params
        .and_then(|params| params.get("EnumProfile"))
        .and_then(Value::as_array)
        .map(|profiles| {
            profiles
                .iter()
                .filter_map(parse_profile)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let active_profile_info = first_object(params.and_then(|params| params.get("Profile")));
    let active_profile = active_profile_info
        .and_then(|profile| profile.get("name"))
        .and_then(value_string)
        .or_else(|| {
            let active_index = active_profile_info?.get("index")?.as_u64()?;
            params?
                .get("EnumProfile")?
                .as_array()?
                .iter()
                .find(|profile| profile.get("index").and_then(Value::as_u64) == Some(active_index))
                .and_then(|profile| profile.get("name"))
                .and_then(value_string)
        });

    Some(AudioCard {
        name,
        driver: None,
        device_api,
        active_profile,
        profiles: available_profiles,
    })
}

fn parse_profile(value: &Value) -> Option<AudioProfile> {
    let name = value.get("name").and_then(value_string).or_else(|| {
        value
            .get("index")
            .and_then(Value::as_u64)
            .map(|index| index.to_string())
    })?;
    let availability = match value.get("available") {
        Some(Value::Bool(true)) => AudioAvailability::Yes,
        Some(Value::Bool(false)) => AudioAvailability::No,
        Some(Value::Number(number)) if number.as_u64() == Some(2) => AudioAvailability::Yes,
        Some(Value::Number(number)) if number.as_u64() == Some(1) => AudioAvailability::No,
        _ => AudioAvailability::Unknown,
    };

    let classes = value.get("classes").unwrap_or(&Value::Null);
    let playback_endpoints = count_class_entries(classes, "Audio/Sink");
    let capture_endpoints = count_class_entries(classes, "Audio/Source");

    Some(AudioProfile {
        name,
        description: value.get("description").and_then(value_string),
        availability,
        playback_endpoints,
        capture_endpoints,
    })
}

fn count_class_entries(classes: &Value, class_name: &str) -> u32 {
    classes
        .get(class_name)
        .and_then(Value::as_array)
        .and_then(|entries| u32::try_from(entries.len()).ok())
        .or_else(|| {
            classes
                .get(class_name)
                .and_then(Value::as_u64)
                .and_then(|count| u32::try_from(count).ok())
        })
        .unwrap_or(0)
}

fn parse_metadata(info: &Value, object: &Value, snapshot: &mut PipeWireSnapshot) {
    let metadata = info
        .get("metadata")
        .or_else(|| object.get("metadata"))
        .and_then(Value::as_array);
    let Some(metadata) = metadata else {
        return;
    };
    let is_default_metadata = info
        .get("props")
        .and_then(|props| string_prop(props, "metadata.name"))
        .as_deref()
        == Some("default");
    if !is_default_metadata {
        return;
    }

    for entry in metadata {
        if entry.get("subject").and_then(Value::as_u64).unwrap_or(0) != 0 {
            continue;
        }
        let Some(key) = entry.get("key").and_then(Value::as_str) else {
            continue;
        };
        let value = entry.get("value").unwrap_or(&Value::Null);
        let name = metadata_node_name(value);
        match key {
            "default.audio.sink" => snapshot.default_sink_name = name,
            "default.audio.source" => snapshot.default_source_name = name,
            _ => {}
        }
    }
}

fn metadata_node_name(value: &Value) -> Option<String> {
    if let Some(name) = value.get("name").and_then(Value::as_str) {
        return Some(name.to_owned());
    }
    let encoded = value.as_str()?;
    if let Ok(parsed) = serde_json::from_str::<Value>(encoded) {
        return parsed
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned);
    }
    Some(encoded.to_owned())
}

fn first_object(value: Option<&Value>) -> Option<&Value> {
    value
        .and_then(Value::as_array)
        .and_then(|values| values.first())
}

fn string_prop(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(value_string)
}

fn value_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_number().map(|number| number.to_string()))
}

fn number_prop_u32(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(parse_u64)
        .and_then(|number| u32::try_from(number).ok())
}

fn number_prop_u8(value: &Value, key: &str) -> Option<u8> {
    value
        .get(key)
        .and_then(parse_u64)
        .and_then(|number| u8::try_from(number).ok())
}

fn parse_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{
        MAX_DUMP_BYTES, PipeWireFailure, drain_limited, metadata_node_name, parse_dump,
        parse_endpoint,
    };

    #[test]
    fn parses_native_pipewire_core_nodes_and_default_metadata() {
        let payload = br#"[
          {"id":0,"type":"PipeWire:Interface:Core","info":{"version":"1.6.10","props":{"core.name":"pipewire-0"}}},
          {"id":5,"type":"PipeWire:Interface:Node","info":{"state":"suspended","props":{"media.class":"Audio/Sink","node.name":"speaker","node.description":"Laptop Speakers","audio.format":"F32LE","audio.rate":"48000","audio.channels":"2"},"params":{"Props":[{"mute":false,"volume":0.75,"channelVolumes":[0.5,1.0]}]}}},
          {"id":6,"type":"PipeWire:Interface:Node","info":{"state":"idle","props":{"media.class":"Audio/Source","node.name":"mic","node.description":"Microphone","audio.format":"S32LE","audio.rate":"48000","audio.channels":"2"},"params":{"Props":[{"mute":false,"volume":1.0}]}}},
          {"id":7,"type":"PipeWire:Interface:Node","info":{"state":"suspended","props":{"media.class":"Audio/Source/Virtual","node.name":"speaker.monitor","audio.channels":"2"}}},
          {"id":8,"type":"PipeWire:Interface:Device","info":{"props":{"media.class":"Video/Device","device.name":"camera"}}},
          {"id":9,"type":"PipeWire:Interface:Device","info":{"props":{"media.class":"Audio/Device","device.name":"audio-card","device.api":"alsa"},"params":{"EnumProfile":[{"index":0,"name":"off","available":true}] ,"Profile":[{"index":0,"name":"off"}]}}},
          {"id":10,"type":"PipeWire:Interface:Metadata","info":{"props":{"metadata.name":"default"},"metadata":[{"subject":0,"key":"default.audio.sink","value":{"name":"speaker"}},{"subject":0,"key":"default.audio.source","value":{"name":"mic"}}]}}
        ]"#;
        let snapshot = parse_dump(payload).expect("fixture should parse");
        assert_eq!(snapshot.core_name.as_deref(), Some("pipewire-0"));
        assert_eq!(snapshot.core_version.as_deref(), Some("1.6.10"));
        assert_eq!(snapshot.sinks.len(), 1);
        assert_eq!(snapshot.sinks[0].volume.minimum, Some(0.5));
        assert_eq!(snapshot.sinks[0].volume.maximum, Some(1.0));
        assert_eq!(snapshot.sinks[0].muted, Some(false));
        assert_eq!(snapshot.default_sink_name.as_deref(), Some("speaker"));
        assert_eq!(snapshot.default_source_name.as_deref(), Some("mic"));
        assert_eq!(snapshot.sources.len(), 2);
        assert!(snapshot.sources.iter().any(|source| !source.is_monitor));
        assert!(snapshot.sources.iter().any(|source| source.is_monitor));
        assert_eq!(snapshot.cards.len(), 1);
        assert_eq!(snapshot.cards[0].name, "audio-card");
        assert_eq!(snapshot.cards[0].device_api.as_deref(), Some("alsa"));
        assert_eq!(snapshot.cards[0].driver, None);
        assert_eq!(snapshot.cards[0].active_profile.as_deref(), Some("off"));
    }

    #[test]
    fn leaves_mute_and_volume_unknown_when_pipewire_does_not_expose_them() {
        let endpoint = parse_endpoint(
            &serde_json::json!({"state":"suspended"}),
            &serde_json::json!({"audio.channels":"2"}),
            "virtual-output".to_owned(),
            None,
        );
        assert_eq!(endpoint.muted, None);
        assert_eq!(endpoint.volume.minimum, None);
        assert_eq!(endpoint.sample_spec.channels, Some(2));
    }

    #[test]
    fn bounds_native_pipewire_dump_output() {
        let input = vec![b'x'; MAX_DUMP_BYTES + 1];
        let (retained, too_large, read_ok) = drain_limited(std::io::Cursor::new(input));
        assert_eq!(retained.len(), MAX_DUMP_BYTES);
        assert!(too_large);
        assert!(read_ok);
    }

    #[test]
    fn rejects_non_pipewire_json_without_core_object() {
        let error = parse_dump(br#"[{"type":"PipeWire:Interface:Node"}]"#)
            .expect_err("missing core object must not be treated as an empty healthy graph");
        assert_eq!(error, PipeWireFailure::ServerUnavailable);
    }

    #[test]
    fn reads_metadata_values_as_objects_or_json_strings() {
        assert_eq!(
            metadata_node_name(&serde_json::json!({"name":"speaker"})).as_deref(),
            Some("speaker")
        );
        assert_eq!(
            metadata_node_name(&serde_json::json!(r#"{"name":"mic"}"#)).as_deref(),
            Some("mic")
        );
    }
}
