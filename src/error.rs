use std::io;
use std::{fmt, fmt::Display};

#[derive(Debug)]
pub enum RatchError {
    IoError(io::Error),
    ParseError(String),
    RegexError(regex::Error),
}

impl From<io::Error> for RatchError {
    fn from(error: io::Error) -> Self {
        RatchError::IoError(error)
    }
}

impl From<regex::Error> for RatchError {
    fn from(error: regex::Error) -> Self {
        RatchError::RegexError(error)
    }
}

impl Display for RatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RatchError::IoError(error) => error.fmt(f),
            RatchError::RegexError(error) => error.fmt(f),
            RatchError::ParseError(error) => error.fmt(f),
        }
    }
}
