//! Single error type for the whole program: a message for the operator.

use std::fmt;

#[derive(Debug)]
pub struct Error(pub String);

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error(e.to_string())
    }
}

/// Return early with a formatted error.
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => { return Err($crate::error::Error(format!($($arg)*))) };
}

/// Attach context to io/other errors.
pub trait Context<T> {
    fn ctx<S: Into<String>>(self, what: S) -> Result<T>;
}

impl<T, E: fmt::Display> Context<T> for std::result::Result<T, E> {
    fn ctx<S: Into<String>>(self, what: S) -> Result<T> {
        self.map_err(|e| Error(format!("{}: {}", what.into(), e)))
    }
}
