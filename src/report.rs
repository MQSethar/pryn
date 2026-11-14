use crate::model::{DiagnosticReport, DiagnosticStatus, SubsystemReport};

pub fn render_json(report: &DiagnosticReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report).map(|mut output| {
        output.push('\n');
        output
    })
}

pub fn render_human(report: &DiagnosticReport) -> String {
    render_human_with_options(report, false)
}

pub fn render_human_with_options(report: &DiagnosticReport, verbose: bool) -> String {
    let mut output = String::new();
    output.push_str("PRYN | System Check\n");
    output.push_str(&format!("Version: {}\n", report.pryn_version));

    let distribution = report
        .system
        .distribution
        .as_deref()
        .unwrap_or("Linux distribution not identified");
    let kernel = report.system.kernel_release.as_deref().unwrap_or("unknown");
    output.push_str(&format!(
        "System: {} | {} | kernel {} | {}\n",
        report.system.operating_system, distribution, kernel, report.system.architecture
    ));
    output.push_str(&format!(
        "Overall result: {}\n",
        human_status(report.overall_status)
    ));
    output.push('\n');

    for subsystem in &report.subsystems {
        render_subsystem(&mut output, subsystem, verbose);
    }

    output.push_str("About these results:\n");
    output.push_str(
        "- 'Not enough information' means PRYN could not confirm whether something is working.\n",
    );
    output.push_str("- 'Incomplete' means some checks are not available in this version yet.\n");
    output.push_str("- PRYN only reads system information. It does not change your settings.\n");
    output.push_str(
        "- A connected Wi-Fi link does not by itself prove that the internet is working.\n",
    );
    output
}

fn render_subsystem(output: &mut String, subsystem: &SubsystemReport, verbose: bool) {
    output.push_str(&format!(
        "{} | {}\n",
        subsystem.subsystem,
        human_status(subsystem.status)
    ));

    if verbose {
        for check in &subsystem.checks {
            output.push_str(&format!(
                "  [{}] {}\n",
                human_status(check.status),
                check.title
            ));
            output.push_str(&format!("      {}\n", check.summary));
            for evidence in &check.evidence {
                output.push_str(&format!("      Detail: {}\n", evidence.detail));
            }
        }
    } else {
        let passed = subsystem
            .checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Pass)
            .count();
        let attention = subsystem
            .checks
            .iter()
            .filter(|check| {
                matches!(
                    check.status,
                    DiagnosticStatus::Problem | DiagnosticStatus::Warning
                )
            })
            .count();
        let unresolved = subsystem
            .checks
            .iter()
            .filter(|check| {
                matches!(
                    check.status,
                    DiagnosticStatus::Unknown
                        | DiagnosticStatus::Partial
                        | DiagnosticStatus::NotImplemented
                )
            })
            .count();
        let not_applicable = subsystem
            .checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::NotApplicable)
            .count();
        output.push_str(&format!(
            "  {} checks: {passed} passed, {attention} need attention, {unresolved} unresolved, {not_applicable} not applicable.\n",
            subsystem.checks.len()
        ));
        for check in subsystem.checks.iter().filter(|check| {
            matches!(
                check.status,
                DiagnosticStatus::Problem
                    | DiagnosticStatus::Warning
                    | DiagnosticStatus::Unknown
                    | DiagnosticStatus::Partial
                    | DiagnosticStatus::NotImplemented
            )
        }) {
            output.push_str(&format!(
                "  [{}] {}: {}\n",
                human_status(check.status),
                check.title,
                check.summary
            ));
        }
    }

    if subsystem.findings.is_empty() {
        output.push_str(
            "  Result: no diagnostic findings were generated from the checks completed.\n",
        );
    } else {
        output.push_str("  What needs attention:\n");
        for finding in &subsystem.findings {
            output.push_str(&format!("    {}\n", finding.title));
            output.push_str(&format!("      {}\n", finding.summary));
            for recommendation in &finding.recommendations {
                output.push_str(&format!("      Next step: {recommendation}\n"));
            }
        }
    }
    output.push('\n');
}

fn human_status(status: DiagnosticStatus) -> &'static str {
    match status {
        DiagnosticStatus::Pass => "OK",
        DiagnosticStatus::Warning => "NEEDS ATTENTION",
        DiagnosticStatus::Problem => "PROBLEM FOUND",
        DiagnosticStatus::Unknown => "NOT ENOUGH INFORMATION",
        DiagnosticStatus::NotApplicable => "NOT NEEDED",
        DiagnosticStatus::NotImplemented => "NOT AVAILABLE YET",
        DiagnosticStatus::Partial => "INCOMPLETE",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{render_human, render_human_with_options, render_json};
    use crate::model::{DiagnosticReport, DiagnosticStatus, SystemInfo};

    #[test]
    fn json_report_contains_a_schema_version_and_status() {
        let report = DiagnosticReport::new(
            1_700_000_000,
            SystemInfo {
                operating_system: "Linux".to_owned(),
                distribution: None,
                kernel_release: None,
                architecture: "x86_64".to_owned(),
            },
            Vec::new(),
        );
        let json = render_json(&report).expect("report should serialize");
        assert!(json.contains("\"schema_version\": 1"));
        assert!(json.contains("\"overall_status\": \"unknown\""));
    }

    #[test]
    fn compact_report_omits_evidence_while_verbose_report_includes_it() {
        use crate::model::{DiagnosticCheck, Evidence, Subsystem, SubsystemReport};

        let subsystem = SubsystemReport::new(
            Subsystem::Audio,
            vec![DiagnosticCheck::new(
                "audio.test",
                "Audio service",
                DiagnosticStatus::Pass,
                "Audio service is available.",
                vec![Evidence::new("test provider", "provider-specific detail")],
            )],
            Vec::new(),
        );
        let report = DiagnosticReport::new(
            1,
            SystemInfo {
                operating_system: "Linux".to_owned(),
                distribution: None,
                kernel_release: None,
                architecture: "x86_64".to_owned(),
            },
            vec![subsystem],
        );

        let compact = render_human_with_options(&report, false);
        let verbose = render_human_with_options(&report, true);
        assert!(!compact.contains("provider-specific detail"));
        assert!(verbose.contains("provider-specific detail"));
    }

    #[test]
    fn human_report_explains_unknown_status() {
        let report = DiagnosticReport::new(
            1,
            SystemInfo {
                operating_system: "Linux".to_owned(),
                distribution: None,
                kernel_release: None,
                architecture: "x86_64".to_owned(),
            },
            Vec::new(),
        );
        let output = render_human(&report);
        assert!(output.contains("NOT ENOUGH INFORMATION"));
        assert!(output.contains("System Check"));
        assert!(!output.contains("sysfs:"));
        assert_eq!(report.overall_status, DiagnosticStatus::Unknown);
    }
}
