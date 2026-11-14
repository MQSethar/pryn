use std::process::ExitCode;

fn main() -> ExitCode {
    match pryn::cli::run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("pryn: {error}");
            ExitCode::from(2)
        }
    }
}
