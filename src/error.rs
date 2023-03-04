use std::fmt::Formatter;

#[derive(Debug)]
pub enum OutputError {
    Format(std::fmt::Error),
    Io(std::io::Error),
    Pdf(lopdf::Error),
    Other(String),
}

/// Result type for this crate
pub type Res<T> = Result<T, OutputError>;

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            OutputError::Format(e) => write!(f, "Formating error: {}", e),
            OutputError::Io(e) => write!(f, "IO error: {}", e),
            OutputError::Pdf(e) => write!(f, "PDF error: {}", e),
            OutputError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl std::error::Error for OutputError {}

impl From<std::fmt::Error> for OutputError {
    fn from(e: std::fmt::Error) -> Self {
        OutputError::Format(e)
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        OutputError::Io(e)
    }
}

impl From<lopdf::Error> for OutputError {
    fn from(e: lopdf::Error) -> Self {
        OutputError::Pdf(e)
    }
}

impl From<&str> for OutputError {
    fn from(e: &str) -> Self {
        OutputError::Other(e.into())
    }
}

impl From<String> for OutputError {
    fn from(e: String) -> Self {
        OutputError::Other(e)
    }
}
