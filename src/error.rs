use std::fmt::Formatter;

#[derive(Debug)]
pub enum OutputError {
    FormatError(std::fmt::Error),
    IoError(std::io::Error),
    PdfError(lopdf::Error),
    OtherError(String),
}

pub type Res<T> = Result<T, OutputError>;

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            OutputError::FormatError(e) => write!(f, "Formating error: {}", e),
            OutputError::IoError(e) => write!(f, "IO error: {}", e),
            OutputError::PdfError(e) => write!(f, "PDF error: {}", e),
            OutputError::OtherError(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl std::error::Error for OutputError {}

impl From<std::fmt::Error> for OutputError {
    fn from(e: std::fmt::Error) -> Self {
        OutputError::FormatError(e)
    }
}

impl From<std::io::Error> for OutputError {
    fn from(e: std::io::Error) -> Self {
        OutputError::IoError(e)
    }
}

impl From<lopdf::Error> for OutputError {
    fn from(e: lopdf::Error) -> Self {
        OutputError::PdfError(e)
    }
}

impl From<&str> for OutputError {
    fn from(e: &str) -> Self {
        OutputError::OtherError(e.into())
    }
}

impl From<String> for OutputError {
    fn from(e: String) -> Self {
        OutputError::OtherError(e)
    }
}
