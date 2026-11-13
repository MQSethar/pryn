//! Read-only audio-server inspection through the PulseAudio client protocol.
//!
//! Native PulseAudio and PipeWire's `pipewire-pulse` compatibility service
//! implement this protocol. This collector uses the protocol's blocking
//! primitives directly so socket read timeouts bound the work without leaving
//! detached reactor threads behind.

use std::{
    ffi::CString,
    fs,
    io::BufReader,
    os::unix::{fs::FileTypeExt, net::UnixStream},
    path::PathBuf,
    time::{Duration, Instant},
};

use pulseaudio::protocol::{self, Command, CommandReply};

const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AudioServerKind {
    PipeWire,
    PipeWireNative,
    PulseAudio,
    PulseAudioCompatible,
    #[default]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioEndpointState {
    Running,
    Idle,
    Suspended,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioAvailability {
    Yes,
    No,
    Unknown,
}

#[derive(Clone, Debug, Default)]
pub struct AudioSampleSpec {
    pub format: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct AudioPort {
    pub name: String,
    pub description: Option<String>,
    pub kind: Option<String>,
    pub availability: AudioAvailability,
}

#[derive(Clone, Debug, Default)]
pub struct AudioVolume {
    /// Lowest channel volume in linear units (1.0 is 100%).
    pub minimum: Option<f32>,
    /// Highest channel volume in linear units (1.0 is 100%).
    pub maximum: Option<f32>,
    /// Arithmetic mean of channel volumes in linear units (1.0 is 100%).
    pub average: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct AudioEndpoint {
    pub name: String,
    pub description: Option<String>,
    pub state: AudioEndpointState,
    /// `None` means the provider did not expose mute state.
    pub muted: Option<bool>,
    pub volume: AudioVolume,
    pub sample_spec: AudioSampleSpec,
    pub ports: Vec<AudioPort>,
    pub active_port: Option<AudioPort>,
}

#[derive(Clone, Debug)]
pub struct AudioSource {
    pub endpoint: AudioEndpoint,
    /// Monitor sources capture playback output, not microphone input.
    pub is_monitor: bool,
}

#[derive(Clone, Debug)]
pub struct AudioProfile {
    pub name: String,
    pub description: Option<String>,
    pub availability: AudioAvailability,
    pub playback_endpoints: u32,
    pub capture_endpoints: u32,
}

#[derive(Clone, Debug)]
pub struct AudioCard {
    pub name: String,
    /// The server-reported card driver/module, when exposed.
    pub driver: Option<String>,
    /// Native PipeWire's device API (for example, `alsa` or `bluez5`).
    pub device_api: Option<String>,
    pub active_profile: Option<String>,
    pub profiles: Vec<AudioProfile>,
}

#[derive(Clone, Debug, Default)]
pub struct AudioServerSnapshot {
    pub server_kind: AudioServerKind,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
    pub default_sink_name: Option<String>,
    pub default_source_name: Option<String>,
    pub server_sample_spec: AudioSampleSpec,
    /// `None` means the query failed; an empty vector means the query
    /// succeeded and the server reported no endpoints.
    pub sinks: Option<Vec<AudioEndpoint>>,
    /// Monitor sources are retained and explicitly marked.
    pub sources: Option<Vec<AudioSource>>,
    pub cards: Option<Vec<AudioCard>>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioServerFailure {
    SocketUnavailable,
    ConnectionFailed,
    AuthenticationFailed,
    RequestRejected,
    QueryTimedOut,
    ProtocolFailure,
}

impl AudioServerFailure {
    pub const fn user_summary(self) -> &'static str {
        match self {
            Self::SocketUnavailable => "No local PulseAudio-compatible server socket was found.",
            Self::ConnectionFailed => "PRYN could not connect to the local audio server.",
            Self::AuthenticationFailed => {
                "The local audio server rejected or could not complete the client handshake."
            }
            Self::RequestRejected => {
                "The audio server rejected one of PRYN's read-only inspection requests."
            }
            Self::QueryTimedOut => {
                "The audio server did not answer its inspection requests within the time limit."
            }
            Self::ProtocolFailure => {
                "The audio server connection succeeded, but its response could not be read."
            }
        }
    }
}

/// Resolve the current user's local PulseAudio-compatible Unix socket.
///
/// Candidate paths are tested independently. A stale `PULSE_SERVER` value
/// must not prevent fallback to `PULSE_RUNTIME_PATH` or `XDG_RUNTIME_DIR`.
fn audio_socket_path_from_env() -> Option<PathBuf> {
    let candidates = std::env::var("PULSE_SERVER")
        .ok()
        .filter(|value| value.starts_with("unix:"))
        .map(|value| PathBuf::from(&value[5..]))
        .into_iter()
        .chain(
            std::env::var("PULSE_RUNTIME_PATH")
                .ok()
                .map(|value| PathBuf::from(value).join("pulse/native")),
        )
        .chain(
            std::env::var("XDG_RUNTIME_DIR")
                .ok()
                .map(|value| PathBuf::from(value).join("pulse/native")),
        );

    first_available_socket(candidates)
}

fn first_available_socket(candidates: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    candidates
        .into_iter()
        .find(|path| fs::metadata(path).is_ok_and(|metadata| metadata.file_type().is_socket()))
}

/// Collect user-session audio state without changing devices, profiles,
/// volume, or mute settings. The blocking handshake is bounded to two seconds
/// per reply; all subsequent introspection requests share a five-second
/// deadline. No client reactor thread is spawned.
pub fn collect() -> Result<AudioServerSnapshot, AudioServerFailure> {
    let socket_path = audio_socket_path_from_env().ok_or(AudioServerFailure::SocketUnavailable)?;
    let socket = UnixStream::connect(&socket_path).map_err(|error| {
        tracing::debug!(error = %error, "could not connect to the audio server socket");
        AudioServerFailure::ConnectionFailed
    })?;

    socket
        .set_read_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|error| {
            tracing::debug!(error = %error, "could not set audio handshake read timeout");
            AudioServerFailure::ConnectionFailed
        })?;
    socket
        .set_write_timeout(Some(HANDSHAKE_TIMEOUT))
        .map_err(|error| {
            tracing::debug!(error = %error, "could not set audio handshake write timeout");
            AudioServerFailure::ConnectionFailed
        })?;

    // The cookie is passed only to the protocol handshake. It is never logged,
    // included in a snapshot, or exposed in diagnostic evidence.
    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| fs::read(path).ok())
        .filter(|bytes| bytes.len() <= 4096)
        .unwrap_or_default();
    let mut reader = BufReader::new(socket);

    let auth = protocol::AuthParams {
        version: protocol::MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };
    let auth_reply = exchange::<protocol::AuthReply>(
        &mut reader,
        0,
        Command::Auth(auth),
        protocol::MAX_VERSION,
        Instant::now() + HANDSHAKE_TIMEOUT,
    )
    .map_err(|error| {
        tracing::debug!(?error, "audio server authentication failed");
        map_handshake_error(error)
    })?;

    let protocol_version = protocol::MAX_VERSION.min(auth_reply.version);
    if protocol_version < protocol::MIN_VERSION {
        return Err(AudioServerFailure::AuthenticationFailed);
    }

    let mut properties = protocol::Props::new();
    properties.set(protocol::Prop::ApplicationName, c"PRYN");
    if let Ok(process_id) = CString::new(std::process::id().to_string()) {
        properties.set(protocol::Prop::ApplicationProcessId, process_id);
    }

    exchange::<protocol::SetClientNameReply>(
        &mut reader,
        1,
        Command::SetClientName(properties),
        protocol_version,
        Instant::now() + HANDSHAKE_TIMEOUT,
    )
    .map_err(|error| {
        tracing::debug!(?error, "audio server client registration failed");
        map_handshake_error(error)
    })?;

    let deadline = Instant::now() + QUERY_TIMEOUT;
    let mut inspection = AudioInspection::new(&mut reader, protocol_version, deadline);
    let server_info = inspection.query::<protocol::ServerInfo>(
        2,
        Command::GetServerInfo,
        "The audio server identity and default-device information could not be read.",
    );
    let sinks = inspection
        .query::<protocol::SinkInfoList>(
            3,
            Command::GetSinkInfoList,
            "Playback output devices could not be enumerated.",
        )
        .map(|items| items.iter().map(convert_sink).collect());
    let sources = inspection
        .query::<protocol::SourceInfoList>(
            4,
            Command::GetSourceInfoList,
            "Audio input devices could not be enumerated.",
        )
        .map(|items| items.iter().map(convert_source).collect());
    let cards = inspection
        .query::<protocol::CardInfoList>(
            5,
            Command::GetCardInfoList,
            "Audio card profiles could not be enumerated.",
        )
        .map(|items| items.iter().map(convert_card).collect());
    let warnings = inspection.into_warnings();

    let server_name = server_info
        .as_ref()
        .and_then(|info| info.server_name.as_deref())
        .map(c_string);
    let server_version = server_info
        .as_ref()
        .and_then(|info| info.server_version.as_deref())
        .map(c_string);
    let default_sink_name = server_info
        .as_ref()
        .and_then(|info| info.default_sink_name.as_deref())
        .map(c_string);
    let default_source_name = server_info
        .as_ref()
        .and_then(|info| info.default_source_name.as_deref())
        .map(c_string);
    let server_sample_spec = server_info
        .as_ref()
        .map(|info| convert_sample_spec(&info.sample_spec))
        .unwrap_or_default();
    let server_kind = classify_server(server_name.as_deref());

    Ok(AudioServerSnapshot {
        server_kind,
        server_name,
        server_version,
        default_sink_name,
        default_source_name,
        server_sample_spec,
        sinks,
        sources,
        cards,
        warnings,
    })
}

struct AudioInspection<'a> {
    reader: &'a mut BufReader<UnixStream>,
    protocol_version: u16,
    deadline: Instant,
    connection_usable: bool,
    warnings: Vec<String>,
}

impl<'a> AudioInspection<'a> {
    fn new(
        reader: &'a mut BufReader<UnixStream>,
        protocol_version: u16,
        deadline: Instant,
    ) -> Self {
        Self {
            reader,
            protocol_version,
            deadline,
            connection_usable: true,
            warnings: Vec::new(),
        }
    }

    fn query<T: CommandReply>(
        &mut self,
        sequence: u32,
        command: Command,
        warning: &'static str,
    ) -> Option<T> {
        if !self.connection_usable {
            return None;
        }

        match exchange(
            &mut *self.reader,
            sequence,
            command,
            self.protocol_version,
            self.deadline,
        ) {
            Ok(reply) => Some(reply),
            Err(failure) => {
                tracing::debug!(?failure, "audio server inspection request failed");
                self.warnings.push(warning.to_owned());
                if failure != AudioServerFailure::RequestRejected {
                    // A timeout, disconnect or malformed reply can leave a late
                    // response on the ordered protocol stream. Do not issue more
                    // requests after that point.
                    self.connection_usable = false;
                }
                None
            }
        }
    }

    fn into_warnings(self) -> Vec<String> {
        self.warnings
    }
}

fn exchange<T: CommandReply>(
    reader: &mut BufReader<UnixStream>,
    sequence: u32,
    command: Command,
    protocol_version: u16,
    deadline: Instant,
) -> Result<T, AudioServerFailure> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(AudioServerFailure::QueryTimedOut);
    }
    let timeout = remaining.min(QUERY_TIMEOUT);
    reader
        .get_ref()
        .set_read_timeout(Some(timeout))
        .map_err(|_| AudioServerFailure::ConnectionFailed)?;
    reader
        .get_ref()
        .set_write_timeout(Some(timeout))
        .map_err(|_| AudioServerFailure::ConnectionFailed)?;

    protocol::write_command_message(reader.get_mut(), sequence, &command, protocol_version)
        .map_err(map_protocol_error)?;
    let read_timeout = deadline.saturating_duration_since(Instant::now());
    if read_timeout.is_zero() {
        return Err(AudioServerFailure::QueryTimedOut);
    }
    reader
        .get_ref()
        .set_read_timeout(Some(read_timeout.min(QUERY_TIMEOUT)))
        .map_err(|_| AudioServerFailure::ConnectionFailed)?;
    let (reply_sequence, reply) =
        protocol::read_reply_message::<T>(reader, protocol_version).map_err(map_protocol_error)?;

    if reply_sequence != sequence {
        return Err(AudioServerFailure::ProtocolFailure);
    }
    Ok(reply)
}

fn map_handshake_error(error: AudioServerFailure) -> AudioServerFailure {
    match error {
        AudioServerFailure::QueryTimedOut => AudioServerFailure::QueryTimedOut,
        AudioServerFailure::ConnectionFailed => AudioServerFailure::ConnectionFailed,
        _ => AudioServerFailure::AuthenticationFailed,
    }
}

fn map_protocol_error(error: protocol::ProtocolError) -> AudioServerFailure {
    match error {
        protocol::ProtocolError::Timeout => AudioServerFailure::QueryTimedOut,
        protocol::ProtocolError::ServerError(protocol::PulseError::Timeout) => {
            AudioServerFailure::QueryTimedOut
        }
        protocol::ProtocolError::ServerError(_) => AudioServerFailure::RequestRejected,
        protocol::ProtocolError::Io(ref io_error)
            if matches!(
                io_error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            AudioServerFailure::QueryTimedOut
        }
        protocol::ProtocolError::Io(_) => AudioServerFailure::ConnectionFailed,
        _ => AudioServerFailure::ProtocolFailure,
    }
}

fn classify_server(name: Option<&str>) -> AudioServerKind {
    let Some(name) = name else {
        return AudioServerKind::Unknown;
    };
    let normalized = name.to_ascii_lowercase();
    if normalized.contains("pipewire") {
        AudioServerKind::PipeWire
    } else if normalized.contains("pulseaudio") {
        AudioServerKind::PulseAudio
    } else {
        AudioServerKind::PulseAudioCompatible
    }
}

fn convert_sink(sink: &protocol::SinkInfo) -> AudioEndpoint {
    AudioEndpoint {
        name: c_string(&sink.name),
        description: sink.description.as_deref().map(c_string),
        state: match sink.state {
            protocol::SinkState::Running => AudioEndpointState::Running,
            protocol::SinkState::Idle => AudioEndpointState::Idle,
            protocol::SinkState::Suspended => AudioEndpointState::Suspended,
        },
        muted: Some(sink.muted),
        volume: convert_volume(&sink.cvolume),
        sample_spec: convert_sample_spec(&sink.sample_spec),
        ports: sink.ports.iter().map(convert_port).collect(),
        active_port: sink.ports.get(sink.active_port).map(convert_port),
    }
}

fn convert_source(source: &protocol::SourceInfo) -> AudioSource {
    AudioSource {
        endpoint: AudioEndpoint {
            name: c_string(&source.name),
            description: source.description.as_deref().map(c_string),
            state: match source.state {
                protocol::SourceState::Running => AudioEndpointState::Running,
                protocol::SourceState::Idle => AudioEndpointState::Idle,
                protocol::SourceState::Suspended => AudioEndpointState::Suspended,
            },
            muted: Some(source.muted),
            volume: convert_volume(&source.cvolume),
            sample_spec: convert_sample_spec(&source.sample_spec),
            ports: source.ports.iter().map(convert_port).collect(),
            active_port: source.ports.get(source.active_port).map(convert_port),
        },
        is_monitor: source.monitor_of_sink_index.is_some() || source.monitor_of_sink_name.is_some(),
    }
}

fn convert_card(card: &protocol::CardInfo) -> AudioCard {
    AudioCard {
        name: c_string(&card.name),
        driver: card.driver.as_deref().map(c_string),
        device_api: None,
        active_profile: card.active_profile.as_deref().map(c_string),
        profiles: card
            .profiles
            .iter()
            .map(|profile| AudioProfile {
                name: c_string(&profile.name),
                description: profile.description.as_deref().map(c_string),
                availability: convert_profile_availability(profile.available),
                playback_endpoints: profile.num_sinks,
                capture_endpoints: profile.num_sources,
            })
            .collect(),
    }
}

fn convert_port(port: &protocol::port_info::PortInfo) -> AudioPort {
    AudioPort {
        name: c_string(&port.name),
        description: port.description.as_deref().map(c_string),
        kind: Some(format!("{:?}", port.port_type)),
        availability: convert_port_availability(port.available),
    }
}

fn convert_sample_spec(spec: &protocol::SampleSpec) -> AudioSampleSpec {
    AudioSampleSpec {
        format: Some(format!("{:?}", spec.format)),
        sample_rate_hz: Some(spec.sample_rate),
        channels: Some(spec.channels),
    }
}

fn convert_volume(volume: &protocol::ChannelVolume) -> AudioVolume {
    let values: Vec<f32> = volume
        .channels()
        .iter()
        .map(|channel| channel.to_linear())
        .filter(|value| value.is_finite())
        .collect();

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

fn convert_port_availability(
    availability: protocol::port_info::PortAvailable,
) -> AudioAvailability {
    match availability {
        protocol::port_info::PortAvailable::Yes => AudioAvailability::Yes,
        protocol::port_info::PortAvailable::No => AudioAvailability::No,
        protocol::port_info::PortAvailable::Unknown => AudioAvailability::Unknown,
    }
}

fn convert_profile_availability(availability: u32) -> AudioAvailability {
    // PulseAudio's pa_available_t values are UNKNOWN=0, NO=1, YES=2.
    match availability {
        1 => AudioAvailability::No,
        2 => AudioAvailability::Yes,
        _ => AudioAvailability::Unknown,
    }
}

fn c_string(value: &std::ffi::CStr) -> String {
    value.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{
        AudioServerFailure, AudioServerKind, classify_server, convert_profile_availability,
        convert_volume, first_available_socket, map_handshake_error, map_protocol_error,
    };
    use pulseaudio::protocol::{ChannelVolume, Volume};

    #[test]
    fn skips_stale_socket_candidates_and_finds_a_later_user_socket() {
        use std::os::unix::net::UnixListener;

        let root = tempfile::tempdir().expect("temporary directory should be created");
        let stale = root.path().join("stale/pulse/native");
        let socket_path = root.path().join("runtime/pulse/native");
        std::fs::create_dir_all(socket_path.parent().expect("socket parent"))
            .expect("socket parent should be created");
        let _listener = UnixListener::bind(&socket_path).expect("test socket should bind");

        assert_eq!(
            first_available_socket([stale, socket_path.clone()]),
            Some(socket_path)
        );
    }

    #[test]
    fn identifies_pipewire_compatibility_server() {
        assert_eq!(
            classify_server(Some("PulseAudio (on PipeWire 1.6.10)")),
            AudioServerKind::PipeWire
        );
    }

    #[test]
    fn identifies_native_pulseaudio_server() {
        assert_eq!(
            classify_server(Some("pulseaudio")),
            AudioServerKind::PulseAudio
        );
    }

    #[test]
    fn maps_card_profile_availability_using_protocol_values() {
        assert_eq!(convert_profile_availability(0), AudioAvailability::Unknown);
        assert_eq!(convert_profile_availability(1), AudioAvailability::No);
        assert_eq!(convert_profile_availability(2), AudioAvailability::Yes);
        assert_eq!(convert_profile_availability(99), AudioAvailability::Unknown);
    }

    #[test]
    fn preserves_transport_failures_during_handshake_error_mapping() {
        assert_eq!(
            map_handshake_error(AudioServerFailure::QueryTimedOut),
            AudioServerFailure::QueryTimedOut,
        );
        assert_eq!(
            map_handshake_error(AudioServerFailure::ConnectionFailed),
            AudioServerFailure::ConnectionFailed,
        );
        assert_eq!(
            map_handshake_error(AudioServerFailure::RequestRejected),
            AudioServerFailure::AuthenticationFailed,
        );
        assert_eq!(
            map_handshake_error(AudioServerFailure::ProtocolFailure),
            AudioServerFailure::AuthenticationFailed,
        );
    }

    #[test]
    fn distinguishes_consumed_server_rejection_from_transport_timeout() {
        assert_eq!(
            map_protocol_error(pulseaudio::protocol::ProtocolError::ServerError(
                pulseaudio::protocol::PulseError::NoEntity,
            )),
            super::AudioServerFailure::RequestRejected,
        );
        assert_eq!(
            map_protocol_error(pulseaudio::protocol::ProtocolError::ServerError(
                pulseaudio::protocol::PulseError::Timeout,
            )),
            super::AudioServerFailure::QueryTimedOut,
        );
    }

    #[test]
    fn converts_protocol_volume_to_linear_levels() {
        let normal = convert_volume(&ChannelVolume::norm(2));
        assert_eq!(normal.minimum, Some(1.0));
        assert_eq!(normal.maximum, Some(1.0));
        assert_eq!(normal.average, Some(1.0));

        let muted = convert_volume(&ChannelVolume::muted(2));
        assert_eq!(muted.maximum, Some(0.0));

        let mut mixed = ChannelVolume::empty();
        mixed.push(Volume::MUTED);
        mixed.push(Volume::NORM);
        let mixed = convert_volume(&mixed);
        assert_eq!(mixed.minimum, Some(0.0));
        assert_eq!(mixed.maximum, Some(1.0));
        assert_eq!(mixed.average, Some(0.5));
    }
}
