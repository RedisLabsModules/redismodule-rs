use crate::context::call_reply::{ErrorCallReply, ErrorReply};
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

impl<'root> From<ErrorCallReply<'root>> for RedisError {
    fn from(err: ErrorCallReply<'root>) -> Self {
        RedisError::String(
            err.to_utf8_string()
                .unwrap_or("can not convert error into String".into()),
        )
    }
}

impl<'root> From<ErrorReply<'root>> for RedisError {
    fn from(err: ErrorReply<'root>) -> Self {
        RedisError::String(
            err.to_utf8_string()
                .unwrap_or("can not convert error into String".into()),
        )
    }
}

impl RedisError {
    #[must_use]
    pub const fn nonexistent_key() -> Self {
        Self::Str("could not perform this operation on a key that doesn't exist")
    }

    #[must_use]
    pub const fn short_read() -> Self {
        Self::Str("short read or OOM loading DB")
    }
}

impl<T: std::error::Error> From<T> for RedisError {
    fn from(e: T) -> Self {
        Self::String(e.to_string())
    }
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = match self {
            Self::WrongArity => "Wrong Arity",
            // remove NUL from the end of raw::REDISMODULE_ERRORMSG_WRONGTYPE
            // before converting &[u8] to &str to ensure CString::new() doesn't
            // panic when this is passed to it.
            Self::WrongType => std::str::from_utf8(
                CStr::from_bytes_with_nul(raw::REDISMODULE_ERRORMSG_WRONGTYPE)
                    .unwrap()
                    .to_bytes(),
            )
            .unwrap(),
            Self::Str(s) => s,
            Self::String(s) => s.as_str(),
        };

        write!(f, "ERR {d}")
    }
}
