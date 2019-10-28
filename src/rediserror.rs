use core::num::{ParseFloatError, ParseIntError};
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::fmt;

#[derive(Debug)]
pub enum RedisError {
    WrongArity,
    Str(&'static str),
    String(String),
}

impl RedisError {
    pub fn nonexistent_key() -> Self {
        RedisError::Str("ERR could not perform this operation on a key that doesn't exist")
    }
}

impl From<&'static str> for RedisError {
    fn from(s: &'static str) -> Self {
        RedisError::Str(s)
    }
}

impl From<String> for RedisError {
    fn from(s: String) -> Self {
        RedisError::String(s)
    }
}

impl From<ParseFloatError> for RedisError {
    fn from(e: ParseFloatError) -> Self {
        RedisError::String(e.to_string())
    }
}

impl From<ParseIntError> for RedisError {
    fn from(e: ParseIntError) -> Self {
        RedisError::String(e.to_string())
    }
}

impl From<FromUtf8Error> for RedisError {
    fn from(e: FromUtf8Error) -> Self {
        RedisError::String(e.to_string())
    }
}

impl From<Utf8Error> for RedisError {
    fn from(e: Utf8Error) -> Self {
        RedisError::String(e.to_string())
    }
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = match self {
            RedisError::WrongArity => "Wrong Arity",            
            RedisError::Str(s) => s,            
            RedisError::String(s) => s.as_str(),            
        };
        
        write!(f, "{}", d)
    }
}

