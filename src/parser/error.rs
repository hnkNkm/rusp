use nom::error::ErrorKind;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedInput(String),
    UnexpectedEof,
    InvalidNumber(String),
    InvalidString(String),
    InvalidType(String),
    UnmatchedParen,
    NomError(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedInput(s) => write!(f, "Unexpected input: {}", s),
            ParseError::UnexpectedEof => write!(f, "Unexpected end of input"),
            ParseError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            ParseError::InvalidString(s) => write!(f, "Invalid string: {}", s),
            ParseError::InvalidType(s) => write!(f, "Invalid type: {}", s),
            ParseError::UnmatchedParen => write!(f, "Unmatched parenthesis"),
            ParseError::NomError(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

impl<'a> From<nom::Err<nom::error::Error<&'a str>>> for ParseError {
    fn from(err: nom::Err<nom::error::Error<&'a str>>) -> Self {
        match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => {
                ParseError::NomError(format!("{:?} at: {}", e.code, e.input))
            }
            nom::Err::Incomplete(_) => ParseError::UnexpectedEof,
        }
    }
}

impl<'a> From<nom::Err<ParseError>> for ParseError {
    fn from(err: nom::Err<ParseError>) -> Self {
        match err {
            nom::Err::Error(e) | nom::Err::Failure(e) => e,
            nom::Err::Incomplete(_) => ParseError::UnexpectedEof,
        }
    }
}

impl<I> nom::error::ParseError<I> for ParseError {
    fn from_error_kind(_input: I, kind: ErrorKind) -> Self {
        ParseError::NomError(format!("Parse error: {:?}", kind))
    }

    fn append(_input: I, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}