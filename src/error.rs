//! Stable error codes and exit codes (spec §11). CONTRACT FILE.
use std::fmt;

pub const EXIT_OK: u8 = 0;
pub const EXIT_VERIFY_FAILED: u8 = 2;
pub const EXIT_USAGE: u8 = 64;
pub const EXIT_DATA: u8 = 65;
pub const EXIT_CANTCREAT: u8 = 73;
pub const EXIT_IO: u8 = 74;
pub const EXIT_TEMPFAIL: u8 = 75;
pub const EXIT_CONFIG: u8 = 78;
pub const EXIT_INTERRUPT: u8 = 130;

#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub exit: u8,
    pub message: String,
    pub hint: Option<String>,
}

impl Error {
    pub fn new(code: &'static str, exit: u8, message: impl Into<String>) -> Self {
        Self { code, exit, message: message.into(), hint: None }
    }
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
    pub fn exit_code(&self) -> u8 {
        self.exit
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub fn usage(msg: impl Into<String>) -> Error {
    Error::new("usage.invalid", EXIT_USAGE, msg)
}
pub fn config(msg: impl Into<String>) -> Error {
    Error::new("config.invalid", EXIT_CONFIG, msg)
}
pub fn data(msg: impl Into<String>) -> Error {
    Error::new("data.invalid", EXIT_DATA, msg)
}
pub fn unsupported_combination(msg: impl Into<String>) -> Error {
    Error::new("matrix.unsupported_combination", EXIT_DATA, msg)
}
pub fn io(msg: impl Into<String>) -> Error {
    Error::new("io.failed", EXIT_IO, msg)
}
pub fn cant_create(msg: impl Into<String>) -> Error {
    Error::new("fs.cannot_create", EXIT_CANTCREAT, msg)
}
pub fn tempfail(msg: impl Into<String>) -> Error {
    Error::new("env.missing_java", EXIT_TEMPFAIL, msg)
}

/// Wizard cancelled by the user (Esc / Ctrl-C) — spec §11.6.
pub fn interrupted(msg: impl Into<String>) -> Error {
    Error::new("interrupt.cancelled", EXIT_INTERRUPT, msg)
}

/// Interactive prompt failed on IO (not a user cancellation).
pub fn input_failed(msg: impl Into<String>) -> Error {
    Error::new("input.failed", EXIT_TEMPFAIL, msg)
}

/// The matrix carries no selectable version at all.
pub fn matrix_empty(msg: impl Into<String>) -> Error {
    Error::new("matrix.empty", EXIT_CONFIG, msg)
}

/// Defensive: a planned path escaped the target root.
pub fn write_io(msg: impl Into<String>) -> Error {
    Error::new("write.io", EXIT_IO, msg)
}
