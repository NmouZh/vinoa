//! vinoa — scaffold Minecraft server plugin projects.
pub mod cli;
pub mod error;
pub mod fsx;
pub mod init;
pub mod matrix;
pub mod precheck;
pub mod report;
pub mod template;
pub mod types;
pub mod verify;
pub mod wizard;

use std::process::ExitCode;

/// Entry point: parse, dispatch, map errors to stable exit codes.
pub fn run() -> ExitCode {
    match cli::parse() {
        Ok(command) => match cli::dispatch(command) {
            Ok(code) => code,
            Err(err) => {
                report::error(&err);
                ExitCode::from(err.exit_code())
            }
        },
        Err(err) => {
            report::error(&err);
            ExitCode::from(err.exit_code())
        }
    }
}
