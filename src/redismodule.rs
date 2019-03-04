use std::ffi::CString;
use std::slice;
use std::str;

pub type RedisResult = Result<RedisValue, RedisError>;

use crate::raw;

#[derive(Debug)]
pub enum RedisError {
    WrongArity,
    Str(&'static str),
    String(String),
}

#[derive(Debug, PartialEq)]
pub enum RedisValue {
    String(String),
    Integer(i64),
    Array(Vec<RedisValue>),
    None,
}

impl From<i64> for RedisValue {
    fn from(i: i64) -> Self {
        RedisValue::Integer(i)
    }
}

impl From<()> for RedisValue {
    fn from(_: ()) -> Self {
        RedisValue::None
    }
}

impl From<String> for RedisValue {
    fn from(s: String) -> Self {
        RedisValue::String(s)
    }
}

impl From<&str> for RedisValue {
    fn from(s: &str) -> Self {
        RedisValue::String(s.to_string())
    }
}

impl From<Vec<i64>> for RedisValue {
    fn from(nums: Vec<i64>) -> Self {
        RedisValue::Array(nums.into_iter().map(RedisValue::Integer).collect())
    }
}

impl From<usize> for RedisValue {
    fn from(i: usize) -> Self {
        (i as i64).into()
    }
}

///////////////////////////////////////////////////

pub trait NextArg: Iterator {
    fn next_string(&mut self) -> Result<String, RedisError>;
    fn next_i64(&mut self) -> Result<i64, RedisError>;
}

impl<T: Iterator<Item = String>> NextArg for T {
    fn next_string(&mut self) -> Result<String, RedisError> {
        self.next().map_or(Err(RedisError::WrongArity), Result::Ok)
    }

    fn next_i64(&mut self) -> Result<i64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), parse_integer)
    }
}

pub fn parse_integer(arg: String) -> Result<i64, RedisError> {
    arg.parse::<i64>()
        .map_err(|_| RedisError::String(format!("Couldn't parse as integer: {}", arg)))
}

///////////////////////////////////////////////////

#[derive(Debug)]
pub struct RedisString {
    ctx: *mut raw::RedisModuleCtx,
    pub inner: *mut raw::RedisModuleString,
}

impl RedisString {
    pub fn create(ctx: *mut raw::RedisModuleCtx, s: &str) -> RedisString {
        let str = CString::new(s).unwrap();
        let inner = unsafe { raw::RedisModule_CreateString.unwrap()(ctx, str.as_ptr(), s.len()) };

        RedisString { ctx, inner }
    }

    pub fn from_ptr<'a>(ptr: *mut raw::RedisModuleString) -> Result<&'a str, str::Utf8Error> {
        let mut len: libc::size_t = 0;
        let bytes = unsafe { raw::RedisModule_StringPtrLen.unwrap()(ptr, &mut len) };

        str::from_utf8(unsafe { slice::from_raw_parts(bytes as *const u8, len) })
    }
}

impl Drop for RedisString {
    fn drop(&mut self) {
        unsafe {
            raw::RedisModule_FreeString.unwrap()(self.ctx, self.inner);
        }
    }
}
