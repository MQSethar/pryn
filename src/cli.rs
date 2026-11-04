use std::io::{self, Write};

use clap::{Parser, Subcommand, ValueEnum};

use crate::{
    diagnostics,
    error::AppError,
    model::Subsystem,
    output::{self, OutputFormat},
};

/// PRYN command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "pryn",
    version,
    about = "Evidence-based Linux system diagnostics",
    long_about = "PRYN inspects available Linux system interfaces and reports evidence-backed diagnostic results.",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Diagnose all supported subsystems or one selected subsystem.
    Diagnose {
        /// Subsystem to diagnose. Omit it to diagnose all supported subsystems.
        #[arg(value_enum)]
        subsystem: Option<SubsystemArgument>,

        /// Output format.
        #[arg(short, long, value_enum, default_value = "text")]
        format: OutputFormatArgument,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SubsystemArgument {
    Wifi,
    Bluetooth,
    Audio,
}

impl From<SubsystemArgument> for Subsystem {
    fn from(value: SubsystemArgument) -> Self {
        match value {
            SubsystemArgument::Wifi => Self::Wifi,
            SubsystemArgument::Bluetooth => Self::Bluetooth,
            SubsystemArgument::Audio => Self::Audio,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormatArgument {
    Text,
    Json,
}

impl From<OutputFormatArgument> for OutputFormat {
    fn from(value: OutputFormatArgument) -> Self {
        match value {
            OutputFormatArgument::Text => Self::Text,
            OutputFormatArgument::Json => Self::Json,
        }
    }
}

/// Execute a parsed CLI invocation.
pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Diagnose { subsystem, format } => {
            let requested_subsystem = subsystem.map(Subsystem::from);
            let report = diagnostics::run(requested_subsystem);
            let rendered = output::render(&report, format.into())?;

            let stdout = io::stdout();
            let mut output = stdout.lock();
            output.write_all(rendered.as_bytes())?;
            output.write_all(b"\n")?;
        }
    }

    Ok(())
}
