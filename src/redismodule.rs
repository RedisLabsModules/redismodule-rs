use core::num::{ParseFloatError, ParseIntError};
use std::ffi::CString;
use std::slice;
use std::str;

pub type RedisResult = Result<RedisValue, RedisError>;

pub use crate::raw;

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

#[derive(Debug, PartialEq)]
pub enum RedisValue {
    SimpleStringStatic(&'static str),
    SimpleString(String),
    BulkString(String),
    Integer(i64),
    Float(f64),
    Array(Vec<RedisValue>),
    None,
}

pub const REDIS_OK: RedisResult = Ok(RedisValue::SimpleStringStatic("OK"));

impl From<i64> for RedisValue {
    fn from(i: i64) -> Self {
        RedisValue::Integer(i)
    }
}

impl From<f64> for RedisValue {
    fn from(f: f64) -> Self {
        RedisValue::Float(f)
    }
}

impl From<()> for RedisValue {
    fn from(_: ()) -> Self {
        RedisValue::None
    }
}

impl From<String> for RedisValue {
    fn from(s: String) -> Self {
        RedisValue::SimpleString(s)
    }
}

impl From<&str> for RedisValue {
    fn from(s: &str) -> Self {
        s.to_string().into()
    }
}

impl From<Option<String>> for RedisValue {
    fn from(s: Option<String>) -> Self {
        match s {
            Some(v) => RedisValue::SimpleString(v),
            None => RedisValue::None,
        }
    }
}

impl From<Vec<RedisValue>> for RedisValue {
    fn from(values: Vec<RedisValue>) -> Self {
        RedisValue::Array(values)
    }
}

impl From<Vec<Option<String>>> for RedisValue {
    fn from(strings: Vec<Option<String>>) -> Self {
        RedisValue::Array(strings.into_iter().map(|v| v.into()).collect())
    }
}

impl From<Vec<String>> for RedisValue {
    fn from(strings: Vec<String>) -> Self {
        RedisValue::Array(strings.into_iter().map(RedisValue::BulkString).collect())
    }
}

impl From<Vec<&String>> for RedisValue {
    fn from(strings: Vec<&String>) -> Self {
        RedisValue::Array(
            strings
                .into_iter()
                .map(|s| RedisValue::BulkString(s.to_string()))
                .collect(),
        )
    }
}

impl From<Vec<i64>> for RedisValue {
    fn from(nums: Vec<i64>) -> Self {
        RedisValue::Array(nums.into_iter().map(RedisValue::Integer).collect())
    }
}

impl From<Vec<f64>> for RedisValue {
    fn from(nums: Vec<f64>) -> Self {
        RedisValue::Array(nums.into_iter().map(RedisValue::Float).collect())
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
    fn next_f64(&mut self) -> Result<f64, RedisError>;
    fn done(&mut self) -> Result<(), RedisError>;
}

impl<T: Iterator<Item = String>> NextArg for T {
    fn next_string(&mut self) -> Result<String, RedisError> {
        self.next().map_or(Err(RedisError::WrongArity), Result::Ok)
    }

    fn next_i64(&mut self) -> Result<i64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| parse_integer(&v))
    }

    fn next_f64(&mut self) -> Result<f64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| parse_float(&v))
    }

    /// Return an error if there are any more arguments
    fn done(&mut self) -> Result<(), RedisError> {
        self.next().map_or(Ok(()), |_| Err(RedisError::WrongArity))
    }
}

pub fn parse_integer(arg: &String) -> Result<i64, RedisError> {
    arg.parse()
        .map_err(|_| RedisError::String(format!("Couldn't parse as integer: {}", arg)))
}

pub fn parse_float(arg: &String) -> Result<f64, RedisError> {
    arg.parse()
        .map_err(|_| RedisError::String(format!("Couldn't parse as float: {}", arg)))
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
