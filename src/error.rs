
use std::io;

#[derive(Debug)]
pub enum RatchError {
    IoError(io::Error),
    ParseError(String),
}

impl From<io::Error> for RatchError {
    fn from(error: io::Error) -> Self {
        RatchError::IoError(error)
    }
}
