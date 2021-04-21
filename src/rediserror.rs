pub use crate::raw;
use std::ffi::CStr;
use std::fmt;

#[derive(Debug)]
pub enum RedisError {
    WrongArity,
    Str(&'static str),
    String(String),
    WrongType,
}

impl RedisError {
    pub fn nonexistent_key() -> Self {
        RedisError::Str("ERR could not perform this operation on a key that doesn't exist")
    }
}

impl<T: std::error::Error> From<T> for RedisError {
    fn from(e: T) -> Self {
        RedisError::String(format!("ERR {}", e))
    }
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = match self {
            RedisError::WrongArity => "Wrong Arity",
            // remove NUL from the end of raw::REDISMODULE_ERRORMSG_WRONGTYPE
            // before converting &[u8] to &str to ensure CString::new() doesn't
            // panic when this is passed to it.
            RedisError::WrongType => std::str::from_utf8(
                CStr::from_bytes_with_nul(raw::REDISMODULE_ERRORMSG_WRONGTYPE)
                    .unwrap()
                    .to_bytes(),
            )
            .unwrap(),
            RedisError::Str(s) => s,
            RedisError::String(s) => s.as_str(),
        };

        write!(f, "{}", d)
    }
}
