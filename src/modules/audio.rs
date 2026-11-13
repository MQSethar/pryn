use crate::model::{
    DiagnosticCheck, DiagnosticStatus, Evidence, Finding, Severity, Subsystem, SubsystemReport,
};
use crate::platform::linux::audio_server::{
    AudioAvailability, AudioEndpoint, AudioEndpointState, AudioServerFailure, AudioServerKind,
    AudioServerSnapshot, AudioSource,
};
use crate::platform::linux::pipewire::{PipeWireFailure, PipeWireSnapshot};
use crate::platform::linux::{AudioSnapshot, LinuxCollector};

pub fn check(collector: &LinuxCollector) -> SubsystemReport {
    // Hardware inventory and user-session audio services are independent. A
    // missing desktop audio server must not hide ALSA hardware discovery.
    let server_probe = crate::platform::linux::audio_server::collect();
    let pipewire_probe = if should_collect_native_pipewire(&server_probe) {
        crate::platform::linux::pipewire::collect()
    } else {
        Err(PipeWireFailure::ServerUnavailable)
    };

    match collector.collect_audio() {
        Ok(snapshot) => evaluate_snapshot_with_pipewire(snapshot, server_probe, pipewire_probe),
        Err(error) => {
            let mut checks = vec![DiagnosticCheck::new(
                "audio.hardware.detected",
                "Sound card inventory",
                DiagnosticStatus::Unknown,
                "PRYN could not inspect the kernel sound-class inventory.",
                vec![Evidence::new("sysfs:/sys/class/sound", error.to_string())],
            )];
            let mut findings = Vec::new();
            let pipewire_expected = should_collect_native_pipewire(&server_probe);
            let resolved_probe = resolve_server_probe(server_probe, &pipewire_probe);
            append_server_diagnostics(resolved_probe, &mut checks, &mut findings);
            append_pipewire_graph_diagnostics(pipewire_probe, pipewire_expected, &mut checks);
            SubsystemReport::new(Subsystem::Audio, checks, findings)
        }
    }
}

#[cfg(test)]
fn evaluate_snapshot(
    snapshot: AudioSnapshot,
    server_probe: Result<AudioServerSnapshot, AudioServerFailure>,
) -> SubsystemReport {
    evaluate_snapshot_with_pipewire(
        snapshot,
        server_probe,
        Err(PipeWireFailure::ServerUnavailable),
    )
}

fn evaluate_snapshot_with_pipewire(
    snapshot: AudioSnapshot,
    server_probe: Result<AudioServerSnapshot, AudioServerFailure>,
    pipewire_probe: Result<PipeWireSnapshot, PipeWireFailure>,
) -> SubsystemReport {
    let pipewire_expected = should_collect_native_pipewire(&server_probe);
    let resolved_probe = resolve_server_probe(server_probe, &pipewire_probe);

    let mut checks = Vec::new();
    let mut findings = Vec::new();

    let hardware_check = if !snapshot.class_available {
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Unknown,
            "The kernel sound class is not exposed. This does not prove that the computer has no audio hardware.",
            vec![Evidence::new(
                "sysfs:/sys/class/sound",
                "The sound class directory is unavailable.",
            )],
        )
    } else if snapshot.cards.is_empty() {
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Unknown,
            "No ALSA sound card was discovered in sysfs.",
            vec![Evidence::new(
                "sysfs:/sys/class/sound",
                "The sound class exists but no `card*` entries were found.",
            )],
        )
    } else {
        let card_names = snapshot
            .cards
            .iter()
            .map(|card| match &card.id {
                Some(id) => format!("{} ({id})", card.kernel_name),
                None => card.kernel_name.clone(),
            })
            .collect::<Vec<_>>();
        DiagnosticCheck::new(
            "audio.hardware.detected",
            "Sound card",
            DiagnosticStatus::Pass,
            format!(
                "Detected {} ALSA sound card(s): {}.",
                card_names.len(),
                card_names.join(", ")
            ),
            snapshot
                .cards
                .iter()
                .map(|card| {
                    Evidence::new(
                        format!("sysfs:/sys/class/sound/{}", card.kernel_name),
                        match &card.id {
                            Some(id) => format!("ALSA card identifier: {id}"),
                            None => {
                                "ALSA card entry discovered; identifier unavailable.".to_owned()
                            }
                        },
                    )
                })
                .collect(),
        )
    };
    checks.push(hardware_check);

    let driver_count = snapshot
        .cards
        .iter()
        .filter(|card| card.driver.is_some())
        .count();
    let driver_status = if !snapshot.cards.is_empty() && driver_count == snapshot.cards.len() {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Unknown
    };
    checks.push(DiagnosticCheck::new(
        "audio.driver.association",
        "Device driver",
        driver_status,
        if driver_count == 0 {
            "No audio card driver association was exposed through sysfs.".to_owned()
        } else {
            format!(
                "Driver associations were exposed for {driver_count} of {} audio card(s).",
                snapshot.cards.len()
            )
        },
        snapshot
            .cards
            .iter()
            .filter_map(|card| {
                card.driver.as_ref().map(|driver| {
                    Evidence::new(
                        format!("sysfs:/class/sound/{}/device/driver", card.kernel_name),
                        format!("Driver: {driver}"),
                    )
                })
            })
            .collect(),
    ));

    append_server_diagnostics(resolved_probe, &mut checks, &mut findings);
    append_pipewire_graph_diagnostics(pipewire_probe, pipewire_expected, &mut checks);

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "audio.collection.completeness",
            "Hardware collection completeness",
            DiagnosticStatus::Warning,
            format!(
                "{} audio hardware attribute(s) could not be read or interpreted.",
                snapshot.warnings.len()
            ),
            snapshot
                .warnings
                .iter()
                .map(|warning| Evidence::new("Linux system information", warning.clone()))
                .collect(),
        ));
    }

    SubsystemReport::new(Subsystem::Audio, checks, findings)
}

fn should_collect_native_pipewire(
    pulse_probe: &Result<AudioServerSnapshot, AudioServerFailure>,
) -> bool {
    match pulse_probe {
        Ok(snapshot) => matches!(
            snapshot.server_kind,
            AudioServerKind::PipeWire | AudioServerKind::PipeWireNative | AudioServerKind::Unknown
        ),
        Err(_) => true,
    }
}

fn resolve_server_probe(
    pulse_probe: Result<AudioServerSnapshot, AudioServerFailure>,
    pipewire_probe: &Result<PipeWireSnapshot, PipeWireFailure>,
) -> Result<AudioServerSnapshot, AudioServerFailure> {
    match pulse_probe {
        Ok(snapshot) => Ok(snapshot),
        Err(failure) => match pipewire_probe {
            Ok(snapshot) => Ok(snapshot.as_audio_server_snapshot()),
            Err(_) => Err(failure),
        },
    }
}

fn append_pipewire_graph_diagnostics(
    probe: Result<PipeWireSnapshot, PipeWireFailure>,
    pipewire_expected: bool,
    checks: &mut Vec<DiagnosticCheck>,
) {
    match probe {
        Ok(snapshot) => {
            let sink_count = snapshot.sinks.len();
            let source_count = snapshot.sources.iter().filter(|source| !source.is_monitor).count();
            let monitor_count = snapshot.sources.len().saturating_sub(source_count);
            let endpoint_count = sink_count + source_count;
            let status = if endpoint_count == 0 {
                DiagnosticStatus::Unknown
            } else {
                DiagnosticStatus::Pass
            };
            let mut details = vec![format!(
                "Native PipeWire graph exposed {sink_count} playback node(s), {source_count} non-monitor capture node(s), and {monitor_count} monitor node(s)."
            )];
            if let Some(name) = snapshot.default_sink_name.as_deref() {
                details.push(format!("Default output metadata: {}.", safe_endpoint_name(name)));
            }
            if let Some(name) = snapshot.default_source_name.as_deref() {
                details.push(format!("Default input metadata: {}.", safe_endpoint_name(name)));
            }
            if let Some(version) = snapshot.core_version.as_deref() {
                details.push(format!("PipeWire core version: {version}."));
            }
            checks.push(DiagnosticCheck::new(
                "audio.pipewire.native_graph",
                "Native PipeWire graph",
                status,
                details.join(" "),
                vec![Evidence::new(
                    "pw-dump JSON",
                    "PRYN inspected native PipeWire Core, Node, Device, and default-metadata objects without changing graph state.",
                )],
            ));
            if !snapshot.warnings.is_empty() {
                checks.push(DiagnosticCheck::new(
                    "audio.pipewire.completeness",
                    "Native PipeWire information",
                    DiagnosticStatus::Warning,
                    format!("{} optional PipeWire object detail(s) could not be interpreted.", snapshot.warnings.len()),
                    snapshot.warnings.iter().map(|warning| Evidence::new("pw-dump JSON", warning.clone())).collect(),
                ));
            }
        }
        Err(failure) if pipewire_expected => checks.push(DiagnosticCheck::new(
            "audio.pipewire.native_graph",
            "Native PipeWire graph",
            DiagnosticStatus::Unknown,
            failure.user_summary(),
            vec![Evidence::new("pw-dump", "The PulseAudio-compatible server identifies PipeWire, but the native graph could not be inspected.")],
        )),
        Err(_) => checks.push(DiagnosticCheck::new(
            "audio.pipewire.native_graph",
            "Native PipeWire graph",
            DiagnosticStatus::NotApplicable,
            "A native PipeWire graph is not required for the detected audio server.",
            Vec::new(),
        )),
    }
}

fn append_server_diagnostics(
    probe: Result<AudioServerSnapshot, AudioServerFailure>,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let snapshot = match probe {
        Ok(snapshot) => snapshot,
        Err(failure) => {
            checks.push(DiagnosticCheck::new(
                "audio.server.available",
                "Audio service",
                DiagnosticStatus::Unknown,
                failure.user_summary(),
                vec![Evidence::new(
                    "Local audio service",
                    "PRYN could not inspect a PulseAudio-compatible socket or native PipeWire graph.",
                )],
            ));
            checks.push(unknown_check(
                "audio.output.inventory",
                "Playback devices",
                "Playback devices could not be inspected because the audio server is unavailable.",
            ));
            checks.push(unknown_check(
                "audio.output.default",
                "Default output",
                "The default playback device could not be determined.",
            ));
            checks.push(unknown_check(
                "audio.input.inventory",
                "Capture devices",
                "Capture devices could not be inspected because the audio server is unavailable.",
            ));
            checks.push(unknown_check(
                "audio.input.default",
                "Default input",
                "The default capture device could not be determined.",
            ));
            checks.push(unknown_check(
                "audio.card.profiles",
                "Audio card profiles",
                "Audio card profiles could not be inspected because the audio server is unavailable.",
            ));
            checks.push(unknown_check(
                "audio.bluetooth.profile",
                "Bluetooth audio profile",
                "Bluetooth audio-card profiles could not be inspected because the audio server is unavailable.",
            ));
            return;
        }
    };

    append_server_identity(&snapshot, checks);
    append_output_diagnostics(&snapshot, checks, findings);
    append_input_diagnostics(&snapshot, checks, findings);
    append_profile_diagnostics(&snapshot, checks, findings);

    if !snapshot.warnings.is_empty() {
        checks.push(DiagnosticCheck::new(
            "audio.server.completeness",
            "Audio service information",
            DiagnosticStatus::Warning,
            format!(
                "{} audio-server inspection request(s) did not return complete information.",
                snapshot.warnings.len()
            ),
            snapshot
                .warnings
                .iter()
                .map(|warning| Evidence::new("Audio service provider", warning.clone()))
                .collect(),
        ));
    }
}

fn append_server_identity(snapshot: &AudioServerSnapshot, checks: &mut Vec<DiagnosticCheck>) {
    let server_label = match snapshot.server_kind {
        AudioServerKind::PipeWire => "PipeWire through its PulseAudio-compatible interface",
        AudioServerKind::PipeWireNative => "native PipeWire",
        AudioServerKind::PulseAudio => "PulseAudio",
        AudioServerKind::PulseAudioCompatible => "a PulseAudio-compatible audio server",
        AudioServerKind::Unknown => "a local audio server",
    };
    let mut details = vec![format!("Connected to {server_label}.")];
    if let Some(version) = snapshot.server_version.as_deref() {
        details.push(format!("Server version: {version}."));
    }
    if let (Some(rate), Some(channels)) = (
        snapshot.server_sample_spec.sample_rate_hz,
        snapshot.server_sample_spec.channels,
    ) {
        details.push(format!(
            "Server default format: {rate} Hz, {channels} channel(s)."
        ));
    }

    let identity_status = if snapshot.server_name.is_some() {
        DiagnosticStatus::Pass
    } else {
        DiagnosticStatus::Unknown
    };
    let (evidence_source, evidence_detail) = if snapshot.server_kind
        == AudioServerKind::PipeWireNative
    {
        (
            "Native PipeWire graph",
            "PRYN inspected PipeWire's read-only JSON graph snapshot without changing server state.",
        )
    } else {
        (
            "PulseAudio client protocol",
            "PRYN completed the local audio-server handshake without changing server state.",
        )
    };
    checks.push(DiagnosticCheck::new(
        "audio.server.available",
        "Audio service",
        identity_status,
        if snapshot.server_name.is_some() {
            details.join(" ")
        } else {
            "The audio protocol connection succeeded, but server identity information could not be read.".to_owned()
        },
        vec![Evidence::new(evidence_source, evidence_detail)],
    ));
}

fn append_output_diagnostics(
    snapshot: &AudioServerSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let Some(sinks) = snapshot.sinks.as_ref() else {
        checks.push(unknown_check(
            "audio.output.inventory",
            "Playback devices",
            "The audio server did not return its playback-device inventory.",
        ));
        checks.push(unknown_check(
            "audio.output.default",
            "Default output",
            "The default playback device could not be verified.",
        ));
        return;
    };

    let suspended_count = sinks
        .iter()
        .filter(|sink| sink.state == AudioEndpointState::Suspended)
        .count();
    let inventory_summary = if sinks.is_empty() {
        "The audio server reports no playback output devices.".to_owned()
    } else {
        format!(
            "Detected {} playback output device(s). {} are currently suspended while idle; this is normal and is not treated as a fault.",
            sinks.len(),
            suspended_count
        )
    };
    let inventory_status = if sinks.is_empty() {
        DiagnosticStatus::Warning
    } else {
        DiagnosticStatus::Pass
    };
    checks.push(DiagnosticCheck::new(
        "audio.output.inventory",
        "Playback devices",
        inventory_status,
        inventory_summary.clone(),
        sinks.iter().map(endpoint_evidence).collect(),
    ));

    let default_sink = snapshot
        .default_sink_name
        .as_deref()
        .and_then(|name| sinks.iter().find(|sink| sink.name == name));

    match default_sink {
        Some(sink) => {
            checks.push(DiagnosticCheck::new(
                "audio.output.default",
                "Default output",
                DiagnosticStatus::Pass,
                format!("The default playback device is {}.", display_endpoint(sink)),
                vec![endpoint_evidence(sink)],
            ));
            append_output_mute_and_volume(sink, checks, findings);
            append_port_check(
                PortCheck {
                    id: "audio.output.port",
                    title: "Output connection",
                    unavailable_summary: "A selected output port reports that it is unavailable. Check whether the speaker or headphone connection is physically attached.",
                    available_summary: "The selected output port is available.",
                    finding_code: "AUDIO-005",
                },
                sink,
                checks,
                findings,
            );
            append_format_check("audio.output.format", "Output format", sink, checks);
        }
        None => {
            let (summary, status, evidence) = if let Some(name) =
                snapshot.default_sink_name.as_deref()
            {
                (
                    format!(
                        "The audio server selected `{}` as the default output, but that device was not present in the returned output inventory.",
                        safe_endpoint_name(name)
                    ),
                    DiagnosticStatus::Warning,
                    vec![Evidence::new(
                        "Audio server",
                        "The default output name did not match an enumerated playback device.",
                    )],
                )
            } else if sinks.is_empty() {
                (
                    "No playback output device is currently exposed by the audio server."
                        .to_owned(),
                    DiagnosticStatus::Warning,
                    Vec::new(),
                )
            } else {
                (
                    "Playback devices exist, but the audio server did not identify a default output.".to_owned(),
                    DiagnosticStatus::Warning,
                    Vec::new(),
                )
            };
            checks.push(DiagnosticCheck::new(
                "audio.output.default",
                "Default output",
                status,
                summary.clone(),
                evidence.clone(),
            ));
            findings.push(Finding::new(
                "AUDIO-001",
                Severity::Low,
                "No usable default output was confirmed",
                summary,
                evidence,
                vec![
                    "Open your desktop sound settings and select the intended speaker or headphone output.".to_owned(),
                    "Check whether the relevant audio card profile exposes a playback output.".to_owned(),
                ],
            ));
            checks.push(unknown_check(
                "audio.output.mute",
                "Output mute state",
                "PRYN cannot inspect mute state without a confirmed default output.",
            ));
            checks.push(unknown_check(
                "audio.output.volume",
                "Output volume",
                "PRYN cannot inspect volume without a confirmed default output.",
            ));
        }
    }
}

fn append_output_mute_and_volume(
    sink: &AudioEndpoint,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let evidence = vec![endpoint_evidence(sink)];
    if sink.muted == Some(true) {
        let summary = "The default playback device is muted. This is a confirmed setting, but may be intentional.";
        checks.push(DiagnosticCheck::new(
            "audio.output.mute",
            "Output mute state",
            DiagnosticStatus::Warning,
            summary,
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-002",
            Severity::Low,
            "Default audio output is muted",
            summary,
            evidence.clone(),
            vec![
                "Unmute the output in your desktop sound settings if you expect to hear audio.".to_owned(),
                "Check the physical keyboard mute key or speaker controls if the mute state keeps returning.".to_owned(),
            ],
        ));
    } else if sink.muted == Some(false) {
        checks.push(DiagnosticCheck::new(
            "audio.output.mute",
            "Output mute state",
            DiagnosticStatus::Pass,
            "The default playback device is not muted.",
            evidence.clone(),
        ));
    } else {
        checks.push(unknown_check(
            "audio.output.mute",
            "Output mute state",
            "The audio provider did not expose the default output mute state.",
        ));
    }

    let Some(maximum) = sink.volume.maximum else {
        checks.push(unknown_check(
            "audio.output.volume",
            "Output volume",
            "The audio server did not expose a usable channel-volume value for the default output.",
        ));
        return;
    };

    if sink.muted == Some(true) {
        checks.push(DiagnosticCheck::new(
            "audio.output.volume",
            "Output volume",
            DiagnosticStatus::NotApplicable,
            "Volume is reported separately, but the output is muted; unmuting is required before its level affects audible output.",
            evidence,
        ));
    } else if maximum <= 0.0001 {
        let summary = "The default playback device volume is set to zero.";
        checks.push(DiagnosticCheck::new(
            "audio.output.volume",
            "Output volume",
            DiagnosticStatus::Warning,
            summary,
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-003",
            Severity::Low,
            "Default output volume is zero",
            summary,
            evidence,
            vec![
                "Raise the default output volume in your desktop sound settings if audio is expected.".to_owned(),
            ],
        ));
    } else if maximum > 1.01 {
        let average = sink.volume.average.unwrap_or(maximum);
        let summary = format!(
            "The default playback device is amplified above 100% (average channel level {:.0}%). Extra amplification can cause distortion.",
            average * 100.0
        );
        checks.push(DiagnosticCheck::new(
            "audio.output.volume",
            "Output volume",
            DiagnosticStatus::Warning,
            summary.clone(),
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-004",
            Severity::Low,
            "Default output is amplified above its normal level",
            summary,
            evidence,
            vec![
                "Keep output at or below 100% when possible; use application volume controls before system-wide amplification.".to_owned(),
            ],
        ));
    } else {
        let average = sink.volume.average.unwrap_or(maximum);
        checks.push(DiagnosticCheck::new(
            "audio.output.volume",
            "Output volume",
            DiagnosticStatus::Pass,
            format!(
                "The default playback level is approximately {:.0}% on average across channels.",
                average * 100.0
            ),
            evidence,
        ));
    }

    append_channel_level_check(
        ChannelLevelCheck {
            id: "audio.output.channel_levels",
            title: "Output channel levels",
            finding_code: "AUDIO-009",
            summary: "One or more playback channels are at zero while other channels are active.",
            recommendation: "Check left/right or surround-channel balance in your desktop sound settings.",
            volume: &sink.volume,
            muted: sink.muted,
        },
        checks,
        findings,
    );
}

fn append_input_diagnostics(
    snapshot: &AudioServerSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let Some(sources) = snapshot.sources.as_ref() else {
        checks.push(unknown_check(
            "audio.input.inventory",
            "Capture devices",
            "The audio server did not return its capture-device inventory.",
        ));
        checks.push(unknown_check(
            "audio.input.default",
            "Default input",
            "The default capture device could not be verified.",
        ));
        return;
    };

    let capture_sources: Vec<&AudioSource> =
        sources.iter().filter(|source| !source.is_monitor).collect();
    let monitor_count = sources.iter().filter(|source| source.is_monitor).count();
    let inventory_summary = if capture_sources.is_empty() {
        format!(
            "No non-monitor capture source was identified. The server also exposes {monitor_count} monitor source(s), which capture playback output and are not counted as input devices."
        )
    } else {
        format!(
            "Detected {} non-monitor capture source(s). {monitor_count} monitor source(s) were excluded because they capture playback output.",
            capture_sources.len()
        )
    };
    checks.push(DiagnosticCheck::new(
        "audio.input.inventory",
        "Capture devices",
        if capture_sources.is_empty() {
            DiagnosticStatus::Unknown
        } else {
            DiagnosticStatus::Pass
        },
        inventory_summary,
        capture_sources
            .iter()
            .map(|source| endpoint_evidence(&source.endpoint))
            .collect(),
    ));

    let default_source = snapshot
        .default_source_name
        .as_deref()
        .and_then(|name| sources.iter().find(|source| source.endpoint.name == name));

    match default_source {
        Some(source) if source.is_monitor => {
            // A monitor source is a loopback of playback. It is not a
            // microphone/capture input and must not trigger microphone advice.
            let evidence = vec![endpoint_evidence(&source.endpoint)];
            checks.push(DiagnosticCheck::new(
                "audio.input.default",
                "Default input",
                DiagnosticStatus::NotApplicable,
                format!(
                    "The server's default source is a playback monitor ({}), not a non-monitor capture input.",
                    display_endpoint(&source.endpoint)
                ),
                evidence.clone(),
            ));
            checks.push(DiagnosticCheck::new(
                "audio.input.mute",
                "Input mute state",
                DiagnosticStatus::NotApplicable,
                "Microphone mute diagnostics do not apply to a playback monitor source.",
                evidence.clone(),
            ));
            checks.push(DiagnosticCheck::new(
                "audio.input.volume",
                "Input volume",
                DiagnosticStatus::NotApplicable,
                "Microphone gain diagnostics do not apply to a playback monitor source.",
                evidence.clone(),
            ));
            checks.push(DiagnosticCheck::new(
                "audio.input.port",
                "Input connection",
                DiagnosticStatus::NotApplicable,
                "A playback monitor does not represent a physical microphone connection.",
                evidence.clone(),
            ));
            checks.push(DiagnosticCheck::new(
                "audio.input.format",
                "Input format",
                DiagnosticStatus::NotApplicable,
                "Capture-input format diagnostics do not apply to a playback monitor source.",
                evidence,
            ));
        }
        Some(source) => {
            checks.push(DiagnosticCheck::new(
                "audio.input.default",
                "Default input",
                DiagnosticStatus::Pass,
                format!(
                    "The audio server has selected a non-monitor capture source as the default: {}.",
                    display_endpoint(&source.endpoint)
                ),
                vec![endpoint_evidence(&source.endpoint)],
            ));
            append_input_mute_and_volume(source, checks, findings);
            append_port_check(
                PortCheck {
                    id: "audio.input.port",
                    title: "Input connection",
                    unavailable_summary: "The selected input port reports that it is unavailable. Check whether the microphone or headset is connected and selected.",
                    available_summary: "The selected input port is available.",
                    finding_code: "AUDIO-007",
                },
                &source.endpoint,
                checks,
                findings,
            );
            append_format_check(
                "audio.input.format",
                "Input format",
                &source.endpoint,
                checks,
            );
        }
        None => {
            let summary = if let Some(name) = snapshot.default_source_name.as_deref() {
                format!(
                    "The audio server selected `{}` as the default input, but it was not present in the returned source inventory.",
                    safe_endpoint_name(name)
                )
            } else if capture_sources.is_empty() {
                "No default capture source was reported. A microphone or virtual input may not be configured; this is not treated as a hardware failure.".to_owned()
            } else {
                "Capture inputs exist, but the audio server did not identify a default input."
                    .to_owned()
            };
            checks.push(DiagnosticCheck::new(
                "audio.input.default",
                "Default input",
                DiagnosticStatus::Unknown,
                summary,
                Vec::new(),
            ));
            checks.push(unknown_check(
                "audio.input.mute",
                "Input mute state",
                "PRYN cannot inspect mute state without a confirmed default input.",
            ));
            checks.push(unknown_check(
                "audio.input.volume",
                "Input volume",
                "PRYN cannot inspect volume without a confirmed default input.",
            ));
        }
    }
}

fn append_input_mute_and_volume(
    source: &AudioSource,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let endpoint = &source.endpoint;
    let evidence = vec![endpoint_evidence(endpoint)];

    if endpoint.muted == Some(true) {
        let summary =
            "The default capture source is muted. This may be an intentional privacy setting.";
        checks.push(DiagnosticCheck::new(
            "audio.input.mute",
            "Input mute state",
            DiagnosticStatus::Warning,
            summary,
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-006",
            Severity::Info,
            "Default capture source is muted",
            summary,
            evidence.clone(),
            vec![
                "Unmute the input only when you intend to use the microphone.".to_owned(),
                "Check the desktop microphone privacy control or hardware mute switch.".to_owned(),
            ],
        ));
    } else if endpoint.muted == Some(false) {
        checks.push(DiagnosticCheck::new(
            "audio.input.mute",
            "Input mute state",
            DiagnosticStatus::Pass,
            "The default capture source is not muted.",
            evidence.clone(),
        ));
    } else {
        checks.push(unknown_check(
            "audio.input.mute",
            "Input mute state",
            "The audio provider did not expose the default input mute state.",
        ));
    }

    let Some(maximum) = endpoint.volume.maximum else {
        checks.push(unknown_check(
            "audio.input.volume",
            "Input volume",
            "The audio server did not expose a usable channel-volume value for the default input.",
        ));
        return;
    };

    if endpoint.muted == Some(true) {
        checks.push(DiagnosticCheck::new(
            "audio.input.volume",
            "Input volume",
            DiagnosticStatus::NotApplicable,
            "The input is muted; its configured volume does not currently reach applications.",
            evidence,
        ));
    } else if maximum <= 0.0001 {
        let summary = "The default capture input volume is set to zero.";
        checks.push(DiagnosticCheck::new(
            "audio.input.volume",
            "Input volume",
            DiagnosticStatus::Warning,
            summary,
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-012",
            Severity::Info,
            "Default capture source level is zero",
            summary,
            evidence,
            vec!["Raise the input level only when you intend to use the microphone.".to_owned()],
        ));
    } else if maximum > 1.01 {
        let average = endpoint.volume.average.unwrap_or(maximum);
        let summary = format!(
            "The default capture input is amplified above 100% (average channel level {:.0}%). Excessive input gain can distort recordings.",
            average * 100.0
        );
        checks.push(DiagnosticCheck::new(
            "audio.input.volume",
            "Input volume",
            DiagnosticStatus::Warning,
            summary.clone(),
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-011",
            Severity::Low,
            "Default capture source is amplified above its normal level",
            summary,
            evidence,
            vec![
                "Reduce microphone gain if recordings sound distorted or clipped.".to_owned(),
                "Prefer an appropriate microphone distance and application-level gain before system-wide amplification.".to_owned(),
            ],
        ));
    } else {
        let average = endpoint.volume.average.unwrap_or(maximum);
        checks.push(DiagnosticCheck::new(
            "audio.input.volume",
            "Input volume",
            DiagnosticStatus::Pass,
            format!(
                "The default capture level is approximately {:.0}% on average across channels.",
                average * 100.0
            ),
            evidence,
        ));
    }

    append_channel_level_check(
        ChannelLevelCheck {
            id: "audio.input.channel_levels",
            title: "Input channel levels",
            finding_code: "AUDIO-010",
            summary: "One or more capture channels are at zero while other channels are active.",
            recommendation: "Check microphone channel levels in your desktop sound settings.",
            volume: &endpoint.volume,
            muted: endpoint.muted,
        },
        checks,
        findings,
    );
}

struct ChannelLevelCheck<'a> {
    id: &'static str,
    title: &'static str,
    finding_code: &'static str,
    summary: &'static str,
    recommendation: &'static str,
    volume: &'a crate::platform::linux::audio_server::AudioVolume,
    muted: Option<bool>,
}

fn append_channel_level_check(
    config: ChannelLevelCheck<'_>,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let ChannelLevelCheck {
        id,
        title,
        finding_code,
        summary,
        recommendation,
        volume,
        muted,
    } = config;
    if muted == Some(true) {
        checks.push(DiagnosticCheck::new(
            id,
            title,
            DiagnosticStatus::NotApplicable,
            "Channel balance is not evaluated while the endpoint is muted.",
            Vec::new(),
        ));
        return;
    }

    let (Some(minimum), Some(maximum)) = (volume.minimum, volume.maximum) else {
        checks.push(unknown_check(
            id,
            title,
            "The audio server did not expose per-channel levels for this endpoint.",
        ));
        return;
    };

    if minimum <= 0.0001 && maximum > 0.001 {
        let evidence = vec![Evidence::new(
            "Audio endpoint",
            format!(
                "Minimum channel level: {:.0}%; maximum channel level: {:.0}%.",
                minimum * 100.0,
                maximum * 100.0
            ),
        )];
        checks.push(DiagnosticCheck::new(
            id,
            title,
            DiagnosticStatus::Warning,
            summary,
            evidence.clone(),
        ));
        findings.push(Finding::new(
            finding_code,
            Severity::Low,
            title,
            summary,
            evidence,
            vec![recommendation.to_owned()],
        ));
    } else {
        checks.push(DiagnosticCheck::new(
            id,
            title,
            DiagnosticStatus::Pass,
            "No channel is configured at zero while another channel is active.",
            Vec::new(),
        ));
    }
}

struct PortCheck {
    id: &'static str,
    title: &'static str,
    unavailable_summary: &'static str,
    available_summary: &'static str,
    finding_code: &'static str,
}

fn append_port_check(
    config: PortCheck,
    endpoint: &AudioEndpoint,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let PortCheck {
        id,
        title,
        unavailable_summary,
        available_summary,
        finding_code,
    } = config;
    let Some(port) = endpoint.active_port.as_ref() else {
        if endpoint.ports.is_empty() {
            checks.push(DiagnosticCheck::new(
                id,
                title,
                DiagnosticStatus::NotApplicable,
                "This endpoint does not expose selectable hardware-port information.",
                Vec::new(),
            ));
        } else {
            checks.push(unknown_check(
                id,
                title,
                "The audio server exposes ports but did not identify an active port.",
            ));
        }
        return;
    };

    let mut port_details = vec![format!(
        "Selected port: {}; availability: {:?}.",
        port.name, port.availability
    )];
    if let Some(kind) = port.kind.as_deref() {
        port_details.push(format!("Port type: {kind}."));
    }
    if let Some(description) = port.description.as_deref() {
        port_details.push(format!("Description: {description}."));
    }
    let evidence = vec![Evidence::new("Audio server", port_details.join(" "))];
    match port.availability {
        AudioAvailability::Yes => checks.push(DiagnosticCheck::new(
            id,
            title,
            DiagnosticStatus::Pass,
            format!("{available_summary} ({})", port.name),
            evidence,
        )),
        AudioAvailability::No => {
            checks.push(DiagnosticCheck::new(
                id,
                title,
                DiagnosticStatus::Warning,
                unavailable_summary,
                evidence.clone(),
            ));
            findings.push(Finding::new(
                finding_code,
                Severity::Low,
                "Selected audio port is unavailable",
                unavailable_summary,
                evidence,
                vec![
                    "Check the physical speaker, headphone, or microphone connection and the selected audio profile.".to_owned(),
                ],
            ));
        }
        AudioAvailability::Unknown => checks.push(unknown_check(
            id,
            title,
            "The selected port exists, but the audio server cannot determine whether it is physically available.",
        )),
    }
}

fn append_format_check(
    id: &'static str,
    title: &'static str,
    endpoint: &AudioEndpoint,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let (Some(rate), Some(channels), Some(format)) = (
        endpoint.sample_spec.sample_rate_hz,
        endpoint.sample_spec.channels,
        endpoint.sample_spec.format.as_deref(),
    ) else {
        checks.push(unknown_check(
            id,
            title,
            "The audio server did not return a complete sample format for this device.",
        ));
        return;
    };
    checks.push(DiagnosticCheck::new(
        id,
        title,
        DiagnosticStatus::Pass,
        format!(
            "The selected device is configured for {format}, {rate} Hz, {channels} channel(s)."
        ),
        vec![endpoint_evidence(endpoint)],
    ));
}

fn append_profile_diagnostics(
    snapshot: &AudioServerSnapshot,
    checks: &mut Vec<DiagnosticCheck>,
    findings: &mut Vec<Finding>,
) {
    let Some(cards) = snapshot.cards.as_ref() else {
        checks.push(unknown_check(
            "audio.card.profiles",
            "Audio card profiles",
            "The audio server did not return sound-card profile information.",
        ));
        append_bluetooth_audio_diagnostics(None, checks);
        return;
    };

    append_bluetooth_audio_diagnostics(Some(cards), checks);

    if cards.is_empty() {
        checks.push(DiagnosticCheck::new(
            "audio.card.profiles",
            "Audio card profiles",
            DiagnosticStatus::NotApplicable,
            "The audio server did not expose any sound-card profile objects. Virtual audio devices may still be available.",
            Vec::new(),
        ));
        return;
    }

    let mut details = Vec::new();
    let mut evidence = Vec::new();
    let mut disabled_cards = Vec::new();
    let mut profile_state_unknown = false;
    for card in cards {
        let active = card.active_profile.as_deref().unwrap_or("unknown");
        if card.active_profile.is_none() {
            profile_state_unknown = true;
        }
        details.push(format!("{}: {active}", display_card_name(card)));
        let profiles = card
            .profiles
            .iter()
            .map(|profile| {
                let description = profile
                    .description
                    .as_deref()
                    .map(|value| format!(": {value}"))
                    .unwrap_or_default();
                format!(
                    "{}{description} ({:?}; {} output(s), {} input(s))",
                    profile.name,
                    profile.availability,
                    profile.playback_endpoints,
                    profile.capture_endpoints
                )
            })
            .collect::<Vec<_>>();
        evidence.push(Evidence::new(
            "Audio server",
            format!(
                "Card {}; driver {}; device API {}; active profile {}; profiles: {}.",
                display_card_name(card),
                card.driver.as_deref().unwrap_or("unknown"),
                card.device_api.as_deref().unwrap_or("not reported"),
                active,
                if profiles.is_empty() {
                    "not reported".to_owned()
                } else {
                    profiles.join(", ")
                }
            ),
        ));
        // A Bluetooth card commonly selects `off` while the headset is
        // disconnected. Its dedicated check reports that state without
        // creating a generic audio-failure finding.
        if active.eq_ignore_ascii_case("off") && !is_bluetooth_audio_card(card) {
            disabled_cards.push(card);
        }
    }

    if !disabled_cards.is_empty() {
        let names = disabled_cards
            .iter()
            .map(|card| display_card_name(card))
            .collect::<Vec<_>>()
            .join(", ");
        let summary = format!(
            "The active profile is `off` for: {names}. This disables the endpoints normally exposed by those cards and may be intentional."
        );
        checks.push(DiagnosticCheck::new(
            "audio.card.profiles",
            "Audio card profiles",
            DiagnosticStatus::Warning,
            summary.clone(),
            evidence.clone(),
        ));
        findings.push(Finding::new(
            "AUDIO-008",
            Severity::Low,
            "An audio card is using its off profile",
            summary,
            evidence,
            vec![
                "Select a profile that exposes the playback or capture devices you need in your desktop sound settings.".to_owned(),
            ],
        ));
    } else {
        checks.push(DiagnosticCheck::new(
            "audio.card.profiles",
            "Audio card profiles",
            if profile_state_unknown {
                DiagnosticStatus::Unknown
            } else {
                DiagnosticStatus::Pass
            },
            format!("Active profiles: {}.", details.join("; ")),
            evidence,
        ));
    }
}

fn unknown_check(id: &'static str, title: &'static str, summary: &'static str) -> DiagnosticCheck {
    DiagnosticCheck::new(id, title, DiagnosticStatus::Unknown, summary, Vec::new())
}

fn endpoint_evidence(endpoint: &AudioEndpoint) -> Evidence {
    let spec = match (
        endpoint.sample_spec.format.as_deref(),
        endpoint.sample_spec.sample_rate_hz,
        endpoint.sample_spec.channels,
    ) {
        (Some(format), Some(rate), Some(channels)) => {
            format!("{format}, {rate} Hz, {channels} channel(s)")
        }
        _ => "sample format unavailable".to_owned(),
    };
    let volume = endpoint
        .volume
        .average
        .map(|value| format!("{:.0}% average channel level", value * 100.0))
        .unwrap_or_else(|| "volume unavailable".to_owned());
    let mute_state = endpoint
        .muted
        .map(|muted| muted.to_string())
        .unwrap_or_else(|| "unknown".to_owned());
    Evidence::new(
        "Audio endpoint",
        format!(
            "{}; state: {:?}; muted: {}; {}; {}.",
            safe_endpoint_name(&endpoint.name), endpoint.state, mute_state, volume, spec
        ),
    )
}

fn display_endpoint(endpoint: &AudioEndpoint) -> String {
    endpoint
        .description
        .as_deref()
        .filter(|description| !description.is_empty())
        .map(safe_endpoint_name)
        .map(|description| format!("{description} ({})", safe_endpoint_name(&endpoint.name)))
        .unwrap_or_else(|| safe_endpoint_name(&endpoint.name))
}

fn is_bluetooth_audio_card(card: &crate::platform::linux::audio_server::AudioCard) -> bool {
    card.device_api.as_deref() == Some("bluez5")
        || card.name.starts_with("bluez_card.")
        || card
            .driver
            .as_deref()
            .is_some_and(|driver| driver.to_ascii_lowercase().contains("bluez5"))
}

fn display_card_name(card: &crate::platform::linux::audio_server::AudioCard) -> String {
    if is_bluetooth_audio_card(card) {
        "Bluetooth audio card".to_owned()
    } else {
        card.name.clone()
    }
}

fn safe_endpoint_name(name: &str) -> String {
    if name.starts_with("bluez_output.") {
        "Bluetooth playback endpoint".to_owned()
    } else if name.starts_with("bluez_input.") {
        "Bluetooth capture endpoint".to_owned()
    } else if name.starts_with("bluez_card.") {
        "Bluetooth audio card".to_owned()
    } else if looks_like_bluetooth_address(name) {
        "Bluetooth device".to_owned()
    } else {
        name.to_owned()
    }
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

fn append_bluetooth_audio_diagnostics(
    cards: Option<&[crate::platform::linux::audio_server::AudioCard]>,
    checks: &mut Vec<DiagnosticCheck>,
) {
    let Some(cards) = cards else {
        checks.push(unknown_check(
            "audio.bluetooth.profile",
            "Bluetooth audio profile",
            "Bluetooth audio-card profiles could not be inspected because the audio provider did not return card information.",
        ));
        return;
    };

    let bluetooth_cards = cards
        .iter()
        .filter(|card| is_bluetooth_audio_card(card))
        .collect::<Vec<_>>();
    if bluetooth_cards.is_empty() {
        checks.push(DiagnosticCheck::new(
            "audio.bluetooth.profile",
            "Bluetooth audio profile",
            DiagnosticStatus::NotApplicable,
            "The audio server did not expose a Bluetooth audio card. A paired Bluetooth device does not necessarily provide an audio profile.",
            Vec::new(),
        ));
        return;
    }

    let profiles = bluetooth_cards
        .iter()
        .map(|card| {
            format!(
                "{}: {}",
                display_card_name(card),
                card.active_profile.as_deref().unwrap_or("unknown")
            )
        })
        .collect::<Vec<_>>();
    let disabled_count = bluetooth_cards
        .iter()
        .filter(|card| {
            card.active_profile
                .as_deref()
                .is_some_and(|profile| profile.eq_ignore_ascii_case("off"))
        })
        .count();
    let unknown_count = bluetooth_cards
        .iter()
        .filter(|card| card.active_profile.is_none())
        .count();
    // `off` is a real observed state, but not proof of failure: BlueZ audio
    // cards often remain registered after a headset disconnects. The
    // independent Bluetooth report carries pairing/connection state.
    let status = if disabled_count == bluetooth_cards.len() && unknown_count == 0 {
        DiagnosticStatus::NotApplicable
    } else if disabled_count > 0 || unknown_count > 0 {
        DiagnosticStatus::Unknown
    } else {
        DiagnosticStatus::Pass
    };
    let summary = format!(
        "The audio provider exposed {} Bluetooth audio card(s): {}. An `off` profile is reported without assuming a fault because the device may be disconnected; check the Bluetooth connection state separately.",
        bluetooth_cards.len(),
        profiles.join("; ")
    );
    checks.push(DiagnosticCheck::new(
        "audio.bluetooth.profile",
        "Bluetooth audio profile",
        status,
        summary,
        bluetooth_cards
            .iter()
            .map(|card| {
                Evidence::new(
                    "Audio server",
                    format!(
                        "Bluetooth audio profile: {}; available profiles: {}.",
                        card.active_profile.as_deref().unwrap_or("unknown"),
                        card.profiles
                            .iter()
                            .map(|profile| profile.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                )
            })
            .collect(),
    ));
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{evaluate_snapshot, should_collect_native_pipewire};
    use crate::model::{DiagnosticStatus, Subsystem};
    use crate::platform::linux::audio_server::{
        AudioCard as ServerAudioCard, AudioEndpoint, AudioEndpointState, AudioSampleSpec,
        AudioServerFailure, AudioServerKind, AudioServerSnapshot, AudioSource, AudioVolume,
    };
    use crate::platform::linux::{AudioCard, AudioSnapshot};

    fn hardware_snapshot() -> AudioSnapshot {
        AudioSnapshot {
            class_available: true,
            cards: vec![AudioCard {
                kernel_name: "card0".to_owned(),
                id: Some("PCH".to_owned()),
                driver: Some("snd_hda_intel".to_owned()),
            }],
            warnings: Vec::new(),
        }
    }

    fn endpoint(name: &str, muted: bool, volume: f32) -> AudioEndpoint {
        AudioEndpoint {
            name: name.to_owned(),
            description: Some(name.to_owned()),
            state: AudioEndpointState::Suspended,
            muted: Some(muted),
            volume: AudioVolume {
                minimum: Some(volume),
                maximum: Some(volume),
                average: Some(volume),
            },
            sample_spec: AudioSampleSpec {
                format: Some("S16Le".to_owned()),
                sample_rate_hz: Some(48_000),
                channels: Some(2),
            },
            ports: Vec::new(),
            active_port: None,
        }
    }

    fn server_snapshot(sink: AudioEndpoint) -> AudioServerSnapshot {
        AudioServerSnapshot {
            server_kind: AudioServerKind::PulseAudio,
            server_name: Some("pulseaudio".to_owned()),
            server_version: Some("15.0".to_owned()),
            default_sink_name: Some(sink.name.clone()),
            default_source_name: None,
            server_sample_spec: AudioSampleSpec::default(),
            sinks: Some(vec![sink]),
            sources: Some(vec![AudioSource {
                endpoint: endpoint("microphone", false, 1.0),
                is_monitor: false,
            }]),
            cards: Some(vec![ServerAudioCard {
                name: "alsa_card.pci".to_owned(),
                driver: Some("module-alsa-card.c".to_owned()),
                device_api: None,
                active_profile: Some("analog-stereo".to_owned()),
                profiles: Vec::new(),
            }]),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn alsa_card_is_detected_when_audio_server_is_unavailable() {
        let report = evaluate_snapshot(
            hardware_snapshot(),
            Err(AudioServerFailure::SocketUnavailable),
        );

        assert_eq!(report.subsystem, Subsystem::Audio);
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.hardware.detected" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.server.available" && check.status == DiagnosticStatus::Unknown
        }));
    }

    #[test]
    fn native_pipewire_is_only_probed_when_pipewire_may_be_the_active_server() {
        let pulse_audio = Ok(server_snapshot(endpoint("speaker", false, 1.0)));
        assert!(!should_collect_native_pipewire(&pulse_audio));

        let mut pipewire = server_snapshot(endpoint("speaker", false, 1.0));
        pipewire.server_kind = AudioServerKind::PipeWire;
        assert!(should_collect_native_pipewire(&Ok(pipewire)));

        let mut unidentified = server_snapshot(endpoint("speaker", false, 1.0));
        unidentified.server_kind = AudioServerKind::Unknown;
        assert!(should_collect_native_pipewire(&Ok(unidentified)));
        assert!(should_collect_native_pipewire(&Err(
            AudioServerFailure::SocketUnavailable
        )));
    }

    #[test]
    fn native_pipewire_is_used_when_no_pulseaudio_compatible_socket_exists() {
        use crate::platform::linux::pipewire::PipeWireSnapshot;

        let mut graph = PipeWireSnapshot::default();
        graph.core_name = Some("pipewire-0".to_owned());
        graph.core_version = Some("1.6.10".to_owned());
        graph.default_sink_name = Some("speaker".to_owned());
        graph.sinks.push(endpoint("speaker", false, 0.75));
        graph.default_source_name = Some("mic".to_owned());
        graph.sources.push(AudioSource {
            endpoint: endpoint("mic", false, 1.0),
            is_monitor: false,
        });

        let report = super::evaluate_snapshot_with_pipewire(
            hardware_snapshot(),
            Err(AudioServerFailure::SocketUnavailable),
            Ok(graph),
        );

        assert!(report.checks.iter().any(|check| {
            check.id == "audio.server.available" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.pipewire.native_graph" && check.status == DiagnosticStatus::Pass
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.output.default" && check.status == DiagnosticStatus::Pass
        }));
    }

    #[test]
    fn suspended_idle_output_is_not_reported_as_a_fault() {
        let report = evaluate_snapshot(
            hardware_snapshot(),
            Ok(server_snapshot(endpoint("speaker", false, 1.0))),
        );

        assert!(report.checks.iter().any(|check| {
            check.id == "audio.output.inventory" && check.status == DiagnosticStatus::Pass
        }));
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| { finding.code == "AUDIO-001" || finding.code == "AUDIO-002" })
        );
    }

    #[test]
    fn zero_volume_default_output_is_reported_without_calling_suspended_a_fault() {
        let report = evaluate_snapshot(
            hardware_snapshot(),
            Ok(server_snapshot(endpoint("speaker", false, 0.0))),
        );

        assert!(report.checks.iter().any(|check| {
            check.id == "audio.output.volume" && check.status == DiagnosticStatus::Warning
        }));
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-003")
        );
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-002")
        );
    }

    #[test]
    fn muted_default_output_does_not_create_a_duplicate_zero_volume_finding() {
        let report = evaluate_snapshot(
            hardware_snapshot(),
            Ok(server_snapshot(endpoint("speaker", true, 0.0))),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-002")
        );
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-003")
        );
    }

    #[test]
    fn monitor_sources_are_not_counted_as_capture_inputs() {
        let mut snapshot = server_snapshot(endpoint("speaker", false, 1.0));
        snapshot.sources = Some(vec![AudioSource {
            endpoint: endpoint("speaker.monitor", true, 0.0),
            is_monitor: true,
        }]);
        snapshot.default_source_name = Some("speaker.monitor".to_owned());

        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));

        let inventory = report
            .checks
            .iter()
            .find(|check| check.id == "audio.input.inventory")
            .expect("input inventory check should exist");
        assert_eq!(inventory.status, DiagnosticStatus::Unknown);
        assert!(inventory.summary.contains("monitor source"));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.input.default" && check.status == DiagnosticStatus::NotApplicable
        }));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.input.mute" && check.status == DiagnosticStatus::NotApplicable
        }));
        assert!(
            !report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-006")
        );
    }

    #[test]
    fn overamplified_default_output_is_flagged() {
        let report = evaluate_snapshot(
            hardware_snapshot(),
            Ok(server_snapshot(endpoint("speaker", false, 1.25))),
        );

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-004")
        );
    }

    #[test]
    fn a_zeroed_playback_channel_is_reported_separately() {
        let mut sink = endpoint("speaker", false, 1.0);
        sink.volume.minimum = Some(0.0);
        sink.volume.maximum = Some(1.0);
        sink.volume.average = Some(0.5);
        let report = evaluate_snapshot(hardware_snapshot(), Ok(server_snapshot(sink)));

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-009")
        );
    }

    #[test]
    fn bluetooth_audio_cards_are_detected_without_exposing_device_addresses() {
        let mut snapshot = server_snapshot(endpoint("bluez_output.AA_BB_CC_DD_EE_FF.a2dp-sink", false, 1.0));
        if let Some(cards) = snapshot.cards.as_mut() {
            cards[0].name = "bluez_card.AA_BB_CC_DD_EE_FF".to_owned();
            cards[0].device_api = Some("bluez5".to_owned());
            cards[0].active_profile = Some("a2dp-sink".to_owned());
        }
        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));
        let bluetooth_check = report
            .checks
            .iter()
            .find(|check| check.id == "audio.bluetooth.profile")
            .expect("Bluetooth audio profile check should exist");

        assert_eq!(bluetooth_check.status, DiagnosticStatus::Pass);
        assert!(!bluetooth_check.summary.contains("AA_BB_CC_DD_EE_FF"));
        assert!(report.checks.iter().any(|check| {
            check.id == "audio.output.default"
                && !check.summary.contains("AA_BB_CC_DD_EE_FF")
        }));
    }

    #[test]
    fn disconnected_bluetooth_card_off_profile_is_not_a_generic_audio_failure() {
        let mut snapshot = server_snapshot(endpoint(
            "bluez_output.AA_BB_CC_DD_EE_FF.a2dp-sink",
            false,
            1.0,
        ));
        if let Some(cards) = snapshot.cards.as_mut() {
            cards[0].name = "bluez_card.AA_BB_CC_DD_EE_FF".to_owned();
            cards[0].device_api = Some("bluez5".to_owned());
            cards[0].active_profile = Some("off".to_owned());
        }

        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));
        let bluetooth_check = report
            .checks
            .iter()
            .find(|check| check.id == "audio.bluetooth.profile")
            .expect("Bluetooth profile check should exist");

        assert_eq!(bluetooth_check.status, DiagnosticStatus::NotApplicable);
        assert!(!report.findings.iter().any(|finding| {
            finding.code == "AUDIO-008"
        }));
    }

    #[test]
    fn bluetooth_address_is_redacted_when_default_endpoint_is_missing() {
        let mut snapshot = server_snapshot(endpoint("speaker", false, 1.0));
        snapshot.default_sink_name = Some("bluez_output.AA_BB_CC_DD_EE_FF.a2dp-sink".to_owned());
        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));

        let default_output = report
            .checks
            .iter()
            .find(|check| check.id == "audio.output.default")
            .expect("default output check should exist");
        assert!(!default_output.summary.contains("AA_BB_CC_DD_EE_FF"));
    }

    #[test]
    fn bluetooth_address_in_device_description_is_redacted() {
        assert_eq!(
            super::safe_endpoint_name("AA:BB:CC:DD:EE:FF"),
            "Bluetooth device"
        );
    }

    #[test]
    fn missing_audio_card_inventory_leaves_bluetooth_profile_unknown() {
        let mut snapshot = server_snapshot(endpoint("speaker", false, 1.0));
        snapshot.cards = None;
        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));

        assert!(report.checks.iter().any(|check| {
            check.id == "audio.bluetooth.profile" && check.status == DiagnosticStatus::Unknown
        }));
    }

    #[test]
    fn inactive_card_profile_is_visible_but_not_assumed_broken() {
        let mut snapshot = server_snapshot(endpoint("speaker", false, 1.0));
        if let Some(cards) = snapshot.cards.as_mut() {
            cards[0].active_profile = Some("off".to_owned());
        }
        let report = evaluate_snapshot(hardware_snapshot(), Ok(snapshot));

        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.code == "AUDIO-008")
        );
    }
}
