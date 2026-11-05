use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use thiserror::Error;
use tracing_subscriber::EnvFilter;

use crate::app;
use crate::model::Subsystem;
use crate::report::{render_human, render_json};

#[derive(Debug, Parser)]
#[command(
    name = "pryn",
    version,
    about = "Check Wi-Fi, Bluetooth, and audio on Linux",
    long_about = "PRYN checks your Linux system and explains what it finds. It only reads system information and does not change system settings."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Check Wi-Fi, Bluetooth, and audio.
    Check(CheckArgs),
}

#[derive(Debug, Args)]
struct CheckArgs {
    /// Check only one area. Omit this argument to check all supported areas.
    #[arg(value_enum)]
    subsystem: Option<SubsystemArgument>,

    /// Choose how PRYN displays the results.
    #[arg(long, value_enum, default_value = "text")]
    format: OutputFormat,

    /// Save the results to a file instead of displaying them.
    #[arg(long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Replace the file if it already exists. Requires --output.
    #[arg(long, requires = "output")]
    force: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("could not prepare the results: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("could not write to standard output: {0}")]
    HelpOutput(#[from] io::Error),

    #[error("output file already exists: `{0}`; use --force to replace it")]
    OutputExists(PathBuf),

    #[error("could not save results to `{path}`: {source}")]
    WriteReport {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub fn run() -> Result<(), CliError> {
    initialize_logging();
    let cli = Cli::parse();

    let Some(command) = cli.command else {
        let mut command = Cli::command();
        command.print_help()?;
        println!();
        return Ok(());
    };

    match command {
        Commands::Check(args) => {
            let target = args.subsystem.map(Subsystem::from);
            let report = app::check(target);
            let output = match args.format {
                OutputFormat::Text => render_human(&report),
                OutputFormat::Json => render_json(&report)?,
            };

            match args.output {
                Some(path) => write_report(&path, &output, args.force)?,
                None => {
                    print!("{output}");
                    io::stdout().flush().map_err(CliError::HelpOutput)?;
                }
            }
        }
    }

    Ok(())
}

fn initialize_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .try_init()
    {
        eprintln!("pryn: logging initialization skipped: {error}");
    }
}

fn write_report(path: &Path, contents: &str, force: bool) -> Result<(), CliError> {
    let mut options = OpenOptions::new();
    options.write(true);

    if force {
        options.create(true).truncate(true);
    } else {
        options.create_new(true);
    }

    let mut file = options.open(path).map_err(|source| {
        if source.kind() == io::ErrorKind::AlreadyExists {
            CliError::OutputExists(path.to_path_buf())
        } else {
            CliError::WriteReport {
                path: path.to_path_buf(),
                source,
            }
        }
    })?;

    file.write_all(contents.as_bytes())
        .map_err(|source| CliError::WriteReport {
            path: path.to_path_buf(),
            source,
        })?;
    file.sync_all().map_err(|source| CliError::WriteReport {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(())
}
