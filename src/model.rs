use serde::{Deserialize, Serialize};
use std::{
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

/// Stable identifier for one of PRYN's supported diagnostic subsystems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Wifi,
    Bluetooth,
    Audio,
}

impl fmt::Display for Subsystem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Wifi => "Wi-Fi",
            Self::Bluetooth => "Bluetooth",
            Self::Audio => "Audio",
        };
        formatter.write_str(value)
    }
}

/// Outcome of an individual diagnostic check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warning,
    Problem,
    Unknown,
    NotApplicable,
}

impl fmt::Display for CheckStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Pass => "PASS",
            Self::Warning => "WARNING",
            Self::Problem => "PROBLEM",
            Self::Unknown => "UNKNOWN",
            Self::NotApplicable => "NOT APPLICABLE",
        };
        formatter.write_str(value)
    }
}

/// Relative urgency of a diagnostic result, independent of its status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Info => "INFO",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        };
        formatter.write_str(value)
    }
}

/// A single source-backed detail supporting a diagnostic result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Interface or file from which the information was collected.
    pub source: String,
    /// Sanitized description of the observed value.
    pub detail: String,
}

impl Evidence {
    pub fn new(source: impl Into<String>, detail: impl Into<String>) -> Self {
        let source = source.into();
        let detail = detail.into();

        Self {
            source: sanitize_terminal_text(&source),
            detail: sanitize_terminal_text(&detail),
        }
    }
}

fn sanitize_terminal_text(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            sanitized.extend(character.escape_default());
        } else {
            sanitized.push(character);
        }
    }
    sanitized
}

/// Result of one deterministic diagnostic check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// Stable machine-readable identifier, for example WIFI.HARDWARE.
    pub code: String,
    pub title: String,
    pub status: CheckStatus,
    pub severity: Severity,
    pub summary: String,
    pub evidence: Vec<Evidence>,
    pub recommendation: Option<String>,
}

impl CheckResult {
    pub fn new(
        code: impl Into<String>,
        title: impl Into<String>,
        status: CheckStatus,
        severity: Severity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            title: title.into(),
            status,
            severity,
            summary: summary.into(),
            evidence: Vec::new(),
            recommendation: None,
        }
    }

    pub fn with_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn with_recommendation(mut self, recommendation: impl Into<String>) -> Self {
        self.recommendation = Some(recommendation.into());
        self
    }
}

/// Aggregated results for one subsystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubsystemReport {
    pub subsystem: Subsystem,
    pub status: CheckStatus,
    pub checks: Vec<CheckResult>,
}

impl SubsystemReport {
    pub fn new(subsystem: Subsystem, checks: Vec<CheckResult>) -> Self {
        let status = aggregate_status(checks.iter().map(|check| check.status));
        Self {
            subsystem,
            status,
            checks,
        }
    }
}

/// Non-sensitive information describing the host that was inspected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_id: Option<String>,
    pub os_name: Option<String>,
    pub os_pretty_name: Option<String>,
    pub os_version_id: Option<String>,
    pub kernel_release: Option<String>,
    pub architecture: String,
}

/// Complete, versioned report emitted by PRYN.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    /// Version of the JSON report contract, not the PRYN application.
    pub schema_version: u16,
    pub pryn_version: String,
    /// Seconds since the Unix epoch in UTC.
    pub generated_at_unix: u64,
    pub system: SystemInfo,
    pub subsystems: Vec<SubsystemReport>,
}

impl DiagnosticReport {
    pub fn new(system: SystemInfo, subsystems: Vec<SubsystemReport>) -> Self {
        let generated_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        Self {
            schema_version: 1,
            pryn_version: env!("CARGO_PKG_VERSION").to_owned(),
            generated_at_unix,
            system,
            subsystems,
        }
    }

    pub fn overall_status(&self) -> CheckStatus {
        aggregate_status(self.subsystems.iter().map(|subsystem| subsystem.status))
    }
}

/// Combine check states without treating an unavailable observation as success.
pub fn aggregate_status(statuses: impl IntoIterator<Item = CheckStatus>) -> CheckStatus {
    let mut saw_status = false;
    let mut saw_pass = false;
    let mut saw_warning = false;
    let mut saw_problem = false;
    let mut saw_unknown = false;
    let mut saw_not_applicable = false;

    for status in statuses {
        saw_status = true;
        match status {
            CheckStatus::Problem => saw_problem = true,
            CheckStatus::Warning => saw_warning = true,
            CheckStatus::Unknown => saw_unknown = true,
            CheckStatus::NotApplicable => saw_not_applicable = true,
            CheckStatus::Pass => saw_pass = true,
        }
    }

    if saw_problem {
        CheckStatus::Problem
    } else if saw_warning {
        CheckStatus::Warning
    } else if !saw_status || saw_unknown {
        CheckStatus::Unknown
    } else if saw_pass {
        CheckStatus::Pass
    } else if saw_not_applicable {
        CheckStatus::NotApplicable
    } else {
        CheckStatus::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{aggregate_status, CheckStatus, Evidence};

    #[test]
    fn problem_has_highest_aggregation_priority() {
        assert_eq!(
            aggregate_status([CheckStatus::Pass, CheckStatus::Warning, CheckStatus::Problem]),
            CheckStatus::Problem
        );
    }

    #[test]
    fn unknown_is_not_reported_as_pass() {
        assert_eq!(
            aggregate_status([CheckStatus::Pass, CheckStatus::Unknown]),
            CheckStatus::Unknown
        );
    }

    #[test]
    fn empty_results_are_unknown() {
        assert_eq!(aggregate_status([]), CheckStatus::Unknown);
    }

    #[test]
    fn not_applicable_only_results_remain_not_applicable() {
        assert_eq!(
            aggregate_status([CheckStatus::NotApplicable, CheckStatus::NotApplicable]),
            CheckStatus::NotApplicable
        );
    }

    #[test]
    fn evidence_escapes_terminal_control_characters() {
        let evidence = Evidence::new("sysfs", "adapter\u{001b}[31mred");
        assert_eq!(evidence.detail, "adapter\\u{1b}[31mred");
    }
}
