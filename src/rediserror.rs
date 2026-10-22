use crate::context::call_reply::{ErrorCallReply, ErrorReply};
pub use crate::raw;
use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt;

#[derive(Debug)]
#[non_exhaustive]
pub enum RedisError {
    WrongArity,
    Str(&'static str),
    String(String),
    WrongType,
    ContainsNul(std::ffi::NulError),
    InvalidUtf8(std::str::Utf8Error),
}

impl RedisError {
    #[must_use]
    pub const fn nonexistent_key() -> Self {
        Self::Str("ERR could not perform this operation on a key that doesn't exist")
    }

    #[must_use]
    pub const fn short_read() -> Self {
        Self::Str("ERR short read or OOM loading DB")
    }

    pub(crate) fn to_str(&self) -> Cow<'_, str> {
        match self {
            RedisError::WrongArity => Cow::Borrowed("Wrong Arity"),
            RedisError::Str(str) => Cow::Borrowed(str),
            RedisError::String(str) => Cow::Borrowed(str.as_str()),
            RedisError::WrongType => {
                const ERR_MSG: &str = {
                    let Ok(str) = CStr::from_bytes_with_nul(raw::REDISMODULE_ERRORMSG_WRONGTYPE)
                    else {
                        panic!()
                    };
                    let Ok(str) = std::str::from_utf8(str.to_bytes()) else {
                        panic!()
                    };

                    str
                };

                Cow::Borrowed(ERR_MSG)
            }
            RedisError::ContainsNul(err) => {
                Cow::Owned(format!("String contained interior NUL byte: {err}"))
            }
            RedisError::InvalidUtf8(err) => Cow::Owned(format!("Invalid UTF8: {err}")),
        }
    }
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl std::error::Error for RedisError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RedisError::ContainsNul(err) => Some(err),
            RedisError::InvalidUtf8(err) => Some(err),
            _ => None,
        }
    }
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

impl From<std::str::Utf8Error> for RedisError {
    fn from(err: std::str::Utf8Error) -> Self {
        RedisError::InvalidUtf8(err)
    }
}
