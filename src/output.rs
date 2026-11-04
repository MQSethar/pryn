use crate::{
    error::AppError,
    model::{CheckResult, DiagnosticReport, SubsystemReport},
};

/// Supported report encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Render a report without writing to stdout or stderr.
pub fn render(report: &DiagnosticReport, format: OutputFormat) -> Result<String, AppError> {
    match format {
        OutputFormat::Text => Ok(render_text(report)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(report)?),
    }
}

fn render_text(report: &DiagnosticReport) -> String {
    let mut output = String::new();
    append_line(
        &mut output,
        format_args!(
            "PRYN {version} - Linux system diagnostics",
            version = report.pryn_version
        ),
    );
    append_line(
        &mut output,
        format_args!("Overall status: {status}", status = report.overall_status()),
    );
    append_line(
        &mut output,
        format_args!(
            "Generated (Unix time): {timestamp}",
            timestamp = report.generated_at_unix
        ),
    );

    let os_name = report
        .system
        .os_pretty_name
        .as_deref()
        .or(report.system.os_name.as_deref())
        .unwrap_or("Unknown Linux distribution");
    append_line(&mut output, format_args!("Operating system: {os_name}"));
    append_line(
        &mut output,
        format_args!(
            "Kernel: {kernel}",
            kernel = report
                .system
                .kernel_release
                .as_deref()
                .unwrap_or("unknown")
        ),
    );
    append_line(
        &mut output,
        format_args!(
            "Architecture: {architecture}",
            architecture = report.system.architecture
        ),
    );

    for subsystem in &report.subsystems {
        render_subsystem(&mut output, subsystem);
    }

    output.trim_end().to_owned()
}

fn append_line(output: &mut String, arguments: std::fmt::Arguments<'_>) {
    output.push_str(&arguments.to_string());
    output.push('\n');
}

fn render_subsystem(output: &mut String, subsystem: &SubsystemReport) {
    append_line(
        output,
        format_args!(
            "\n{subsystem} [{status}]",
            subsystem = subsystem.subsystem,
            status = subsystem.status
        ),
    );

    for check in &subsystem.checks {
        render_check(output, check);
    }
}

fn render_check(output: &mut String, check: &CheckResult) {
    append_line(
        output,
        format_args!(
            "  [{status}] {code} - {title}",
            status = check.status,
            code = check.code,
            title = check.title
        ),
    );
    append_line(
        output,
        format_args!("    {summary}", summary = check.summary),
    );
    append_line(
        output,
        format_args!("    Severity: {severity}", severity = check.severity),
    );

    for evidence in &check.evidence {
        append_line(
            output,
            format_args!(
                "    Evidence: {source} - {detail}",
                source = evidence.source,
                detail = evidence.detail
            ),
        );
    }

    if let Some(recommendation) = &check.recommendation {
        append_line(
            output,
            format_args!("    Recommendation: {recommendation}"),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        model::{
            CheckResult, CheckStatus, DiagnosticReport, Severity, Subsystem, SubsystemReport,
            SystemInfo,
        },
        output::{render, OutputFormat},
    };

    fn sample_report() -> DiagnosticReport {
        DiagnosticReport {
            schema_version: 1,
            pryn_version: "0.1.0".to_owned(),
            generated_at_unix: 1_700_000_000,
            system: SystemInfo {
                os_id: Some("ubuntu".to_owned()),
                os_name: Some("Ubuntu".to_owned()),
                os_pretty_name: Some("Ubuntu Test".to_owned()),
                os_version_id: Some("24.04".to_owned()),
                kernel_release: Some("6.8.0-test".to_owned()),
                architecture: "x86_64".to_owned(),
            },
            subsystems: vec![SubsystemReport::new(
                Subsystem::Wifi,
                vec![CheckResult::new(
                    "WIFI.TEST",
                    "Test check",
                    CheckStatus::Pass,
                    Severity::Info,
                    "The test passed.",
                )],
            )],
        }
    }

    #[test]
    fn json_output_is_valid_and_contains_schema_version() {
        let rendered = render(&sample_report(), OutputFormat::Json)
            .expect("JSON serialization should succeed");
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("rendered output should be valid JSON");
        assert_eq!(parsed["schema_version"], 1);
        assert_eq!(parsed["subsystems"][0]["subsystem"], "wifi");
    }

    #[test]
    fn text_output_contains_status_and_evidence_sections() {
        let rendered = render(&sample_report(), OutputFormat::Text)
            .expect("text rendering should succeed");
        assert!(rendered.contains("PRYN 0.1.0"));
        assert!(rendered.contains("WIFI.TEST"));
        assert!(rendered.contains("Overall status: PASS"));
    }
}
