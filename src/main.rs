use std::process::ExitCode;

fn main() -> ExitCode {
    match pryn::cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pryn: {error}");
            ExitCode::FAILURE
        }
    }
}
