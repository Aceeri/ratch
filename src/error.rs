
use std::io;
use std::{fmt, fmt::Display};

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

impl Display for RatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RatchError::IoError(io_error) => io_error.fmt(f),
            RatchError::ParseError(parse_error) => parse_error.fmt(f)
        }
    }
}
