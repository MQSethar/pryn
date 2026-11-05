use serde::Serialize;
use std::fmt;

pub const REPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Pass,
    Warning,
    Problem,
    Unknown,
    NotApplicable,
    NotImplemented,
    Partial,
}

impl DiagnosticStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Warning => "WARNING",
            Self::Problem => "PROBLEM",
            Self::Unknown => "UNKNOWN",
            Self::NotApplicable => "NOT APPLICABLE",
            Self::NotImplemented => "NOT IMPLEMENTED",
            Self::Partial => "PARTIAL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Evidence {
    pub source: String,
    pub detail: String,
}

impl Evidence {
    pub fn new(source: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticCheck {
    pub id: String,
    pub title: String,
    pub status: DiagnosticStatus,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

impl DiagnosticCheck {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        status: DiagnosticStatus,
        summary: impl Into<String>,
        evidence: Vec<Evidence>,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status,
            summary: summary.into(),
            evidence,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    pub title: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommendations: Vec<String>,
}

impl Finding {
    pub fn new(
        code: impl Into<String>,
        severity: Severity,
        title: impl Into<String>,
        summary: impl Into<String>,
        evidence: Vec<Evidence>,
        recommendations: Vec<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            title: title.into(),
            summary: summary.into(),
            evidence,
            recommendations,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SubsystemReport {
    pub subsystem: Subsystem,
    pub status: DiagnosticStatus,
    pub checks: Vec<DiagnosticCheck>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<Finding>,
}

impl SubsystemReport {
    pub fn new(subsystem: Subsystem, checks: Vec<DiagnosticCheck>, findings: Vec<Finding>) -> Self {
        let status = aggregate_status(checks.iter().map(|check| check.status));
        Self {
            subsystem,
            status,
            checks,
            findings,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SystemInfo {
    pub operating_system: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_release: Option<String>,
    pub architecture: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiagnosticReport {
    pub schema_version: u32,
    pub pryn_version: String,
    pub generated_at_unix: u64,
    pub system: SystemInfo,
    pub overall_status: DiagnosticStatus,
    pub subsystems: Vec<SubsystemReport>,
}

impl DiagnosticReport {
    pub fn new(
        generated_at_unix: u64,
        system: SystemInfo,
        subsystems: Vec<SubsystemReport>,
    ) -> Self {
        let overall_status = aggregate_status(subsystems.iter().map(|report| report.status));
        Self {
            schema_version: REPORT_SCHEMA_VERSION,
            pryn_version: env!("CARGO_PKG_VERSION").to_owned(),
            generated_at_unix,
            system,
            overall_status,
            subsystems,
        }
    }
}

pub fn aggregate_status(statuses: impl IntoIterator<Item = DiagnosticStatus>) -> DiagnosticStatus {
    let collected: Vec<DiagnosticStatus> = statuses.into_iter().collect();

    if collected.is_empty() {
        return DiagnosticStatus::Unknown;
    }

    if collected.contains(&DiagnosticStatus::Problem) {
        DiagnosticStatus::Problem
    } else if collected.contains(&DiagnosticStatus::Warning) {
        DiagnosticStatus::Warning
    } else if collected.contains(&DiagnosticStatus::Unknown) {
        DiagnosticStatus::Unknown
    } else if collected.contains(&DiagnosticStatus::Partial)
        || collected.contains(&DiagnosticStatus::NotImplemented)
    {
        DiagnosticStatus::Partial
    } else if collected
        .iter()
        .all(|status| *status == DiagnosticStatus::NotApplicable)
    {
        DiagnosticStatus::NotApplicable
    } else {
        DiagnosticStatus::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::{DiagnosticStatus, aggregate_status};

    #[test]
    fn problem_has_highest_priority() {
        let status = aggregate_status([
            DiagnosticStatus::Pass,
            DiagnosticStatus::Warning,
            DiagnosticStatus::Problem,
            DiagnosticStatus::Unknown,
        ]);
        assert_eq!(status, DiagnosticStatus::Problem);
    }

    #[test]
    fn unknown_is_not_reported_as_success() {
        let status = aggregate_status([
            DiagnosticStatus::Pass,
            DiagnosticStatus::Unknown,
            DiagnosticStatus::NotImplemented,
        ]);
        assert_eq!(status, DiagnosticStatus::Unknown);
    }

    #[test]
    fn unimplemented_checks_make_a_report_partial() {
        let status = aggregate_status([DiagnosticStatus::Pass, DiagnosticStatus::NotImplemented]);
        assert_eq!(status, DiagnosticStatus::Partial);
    }

    #[test]
    fn empty_checks_are_unknown() {
        assert_eq!(aggregate_status([]), DiagnosticStatus::Unknown);
    }
}
