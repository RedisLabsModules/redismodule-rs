use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::slice;
use std::str;
use std::string::FromUtf8Error;

pub use crate::raw;
pub use crate::rediserror::RedisError;
pub use crate::redisvalue::RedisValue;

pub type RedisResult = Result<RedisValue, RedisError>;

pub const REDIS_OK: RedisResult = Ok(RedisValue::SimpleStringStatic("OK"));
pub const TYPE_METHOD_VERSION: u64 = raw::REDISMODULE_TYPE_METHOD_VERSION as u64;

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

///////////////////////////////////////////////////

#[derive(Debug)]
pub struct RedisBuffer {
    buffer: *mut c_char,
    len: usize,
}

impl RedisBuffer {
    pub fn new(buffer: *mut c_char, len: usize) -> RedisBuffer {
        RedisBuffer { buffer, len }
    }

    pub fn to_string(&self) -> Result<String, FromUtf8Error> {
        String::from_utf8(self.as_ref().to_vec())
    }
}

impl AsRef<[u8]> for RedisBuffer {
    fn as_ref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.buffer as *const u8, self.len) }
    }
}

impl Drop for RedisBuffer {
    fn drop(&mut self) {
        unsafe {
            raw::RedisModule_Free.unwrap()(self.buffer as *mut c_void);
        }
    }
}
