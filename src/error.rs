use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::Path;

pub type Result<T> = std::result::Result<T, SafeArcError>;

#[derive(Debug)]
pub enum SafeArcError {
    Io { context: String, source: io::Error },
    Config(String),
    Backend(String),
    Policy(String),
    Sandbox(String),
    Unsupported(String),
    Usage(String),
}

impl SafeArcError {
    pub fn io(context: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub fn io_path(action: &str, path: &Path, source: io::Error) -> Self {
        Self::io(format!("{action}: {}", path.display()), source)
    }
}

impl Display for SafeArcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { context, source } => write!(f, "{context}: {source}"),
            Self::Config(message) => write!(f, "configuration error: {message}"),
            Self::Backend(message) => write!(f, "backend error: {message}"),
            Self::Policy(message) => write!(f, "archive rejected: {message}"),
            Self::Sandbox(message) => write!(f, "sandbox error: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported operation: {message}"),
            Self::Usage(message) => write!(f, "usage error: {message}"),
        }
    }
}

impl Error for SafeArcError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
