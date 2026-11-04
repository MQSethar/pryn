#![forbid(unsafe_code)]

#[cfg(not(target_os = "linux"))]
compile_error!("PRYN currently supports Linux targets only.");

use std::process::ExitCode;

use clap::Parser;
use tracing_subscriber::EnvFilter;

use pryn::{cli, error::AppError};

fn main() -> ExitCode {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let arguments = cli::Cli::parse();

    match cli::run(arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(AppError::Io(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("pryn: {error}");
            ExitCode::FAILURE
        }
    }
}
