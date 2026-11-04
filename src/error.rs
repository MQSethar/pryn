use thiserror::Error;

/// Errors that prevent PRYN from producing a diagnostic report.
#[derive(Debug, Error)]
pub enum AppError {
    /// JSON serialization failed.
    #[error("could not serialize diagnostic report: {0}")]
    JsonSerialization(#[from] serde_json::Error),

    /// Writing the rendered report to standard output failed.
    #[error("could not write diagnostic report: {0}")]
    Io(#[from] std::io::Error),
}
