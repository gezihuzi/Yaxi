use crate::display;

#[derive(Debug)]
pub enum Error {
    SelectionTimeout,
    SelectionNoData,
    InvalidProperty,
    Timeout,
    SaveFailed,
    ConversionFailure,
    FailedToLock,
    FromUtf8Error(std::string::FromUtf8Error),
    Display(display::error::Error),
    RwLock(String),
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SelectionTimeout => write!(f, "SelectionTimeout"),
            Error::SelectionNoData => write!(f, "SelectionNoData"),
            Error::InvalidProperty => write!(f, "InvalidProperty"),
            Error::Timeout => write!(f, "Timeout"),
            Error::SaveFailed => write!(f, "SaveFailed"),
            Error::ConversionFailure => write!(f, "ConversionFailure"),
            Error::FailedToLock => write!(f, "FailedToLock"),
            Error::FromUtf8Error(e) => write!(f, "FromUtf8Error: {}", e),
            Error::Display(e) => write!(f, "Display: {}", e),
            Error::RwLock(e) => write!(f, "RwLock: {}", e),
            Error::Other(e) => write!(f, "Other: {}", e),
        }
    }
}

impl From<display::error::Error> for Error {
    fn from(e: display::error::Error) -> Self {
        Error::Display(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::FromUtf8Error(e)
    }
}

impl std::error::Error for Error {}
