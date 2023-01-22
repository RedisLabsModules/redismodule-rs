use std::borrow::Borrow;
use std::convert::TryFrom;
use std::ffi::CString;
use std::fmt;
use std::fmt::Display;
use std::os::raw::{c_char, c_int, c_void};
use std::slice;
use std::str;
use std::str::Utf8Error;
use std::string::FromUtf8Error;

pub use crate::raw;
pub use crate::rediserror::RedisError;
pub use crate::redisvalue::RedisValue;

pub type RedisResult = Result<RedisValue, RedisError>;

pub const REDIS_OK: RedisResult = Ok(RedisValue::SimpleStringStatic("OK"));
pub const TYPE_METHOD_VERSION: u64 = raw::REDISMODULE_TYPE_METHOD_VERSION as u64;

pub trait NextArg {
    fn next_arg(&mut self) -> Result<RedisString, RedisError>;
    fn next_string(&mut self) -> Result<String, RedisError>;
    fn next_str<'a>(&mut self) -> Result<&'a str, RedisError>;
    fn next_i64(&mut self) -> Result<i64, RedisError>;
    fn next_u64(&mut self) -> Result<u64, RedisError>;
    fn next_f64(&mut self) -> Result<f64, RedisError>;
    fn done(&mut self) -> Result<(), RedisError>;
}

impl<T> NextArg for T
where
    T: Iterator<Item = RedisString>,
{
    #[inline]
    fn next_arg(&mut self) -> Result<RedisString, RedisError> {
        self.next().ok_or(RedisError::WrongArity)
    }

    #[inline]
    fn next_string(&mut self) -> Result<String, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| Ok(v.to_string_lossy()))
    }

    #[inline]
    fn next_str<'a>(&mut self) -> Result<&'a str, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| v.try_as_str())
    }

    #[inline]
    fn next_i64(&mut self) -> Result<i64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| v.parse_integer())
    }

    #[inline]
    fn next_u64(&mut self) -> Result<u64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| v.parse_unsigned_integer())
    }

    #[inline]
    fn next_f64(&mut self) -> Result<f64, RedisError> {
        self.next()
            .map_or(Err(RedisError::WrongArity), |v| v.parse_float())
    }

    /// Return an error if there are any more arguments
    #[inline]
    fn done(&mut self) -> Result<(), RedisError> {
        self.next().map_or(Ok(()), |_| Err(RedisError::WrongArity))
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn decode_args(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> Vec<RedisString> {
    unsafe { slice::from_raw_parts(argv, argc as usize) }
        .iter()
        .map(|&arg| RedisString::new(ctx, arg))
        .collect()
}

///////////////////////////////////////////////////

#[derive(Debug)]
pub struct RedisString {
    ctx: *mut raw::RedisModuleCtx,
    pub inner: *mut raw::RedisModuleString,
}

impl RedisString {
    pub(crate) fn take(mut self) -> *mut raw::RedisModuleString {
        let inner = self.inner;
        self.inner = std::ptr::null_mut();
        inner
    }

    pub fn new(ctx: *mut raw::RedisModuleCtx, inner: *mut raw::RedisModuleString) -> Self {
        raw::string_retain_string(ctx, inner);
        Self { ctx, inner }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn create(ctx: *mut raw::RedisModuleCtx, s: &str) -> Self {
        let str = CString::new(s).unwrap();
        let inner = unsafe { raw::RedisModule_CreateString.unwrap()(ctx, str.as_ptr(), s.len()) };

        Self { ctx, inner }
    }

    pub fn create_from_slice(ctx: *mut raw::RedisModuleCtx, s: &[u8]) -> Self {
        let inner = unsafe {
            raw::RedisModule_CreateString.unwrap()(ctx, s.as_ptr() as *const c_char, s.len())
        };

        Self { ctx, inner }
    }

    pub fn from_redis_module_string(
        ctx: *mut raw::RedisModuleCtx,
        inner: *mut raw::RedisModuleString,
    ) -> Self {
        // Need to avoid string_retain_string
        Self { ctx, inner }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn from_ptr<'a>(ptr: *const raw::RedisModuleString) -> Result<&'a str, Utf8Error> {
        str::from_utf8(Self::string_as_slice(ptr))
    }

    pub fn append(&mut self, s: &str) -> raw::Status {
        raw::string_append_buffer(self.ctx, self.inner, s)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let mut len: usize = 0;
        raw::string_ptr_len(self.inner, &mut len);
        len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        let mut len: usize = 0;
        raw::string_ptr_len(self.inner, &mut len);
        len == 0
    }

    pub fn try_as_str<'a>(&self) -> Result<&'a str, RedisError> {
        Self::from_ptr(self.inner).map_err(|_| RedisError::Str("Couldn't parse as UTF-8 string"))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        Self::string_as_slice(self.inner)
    }

    fn string_as_slice<'a>(ptr: *const raw::RedisModuleString) -> &'a [u8] {
        let mut len: libc::size_t = 0;
        let bytes = unsafe { raw::RedisModule_StringPtrLen.unwrap()(ptr, &mut len) };

        unsafe { slice::from_raw_parts(bytes.cast::<u8>(), len) }
    }

    /// Performs lossy conversion of a `RedisString` into an owned `String. This conversion
    /// will replace any invalid UTF-8 sequences with U+FFFD REPLACEMENT CHARACTER, which
    /// looks like this: �.
    ///
    /// # Panics
    ///
    /// Will panic if `RedisModule_StringPtrLen` is missing in redismodule.h
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self.as_slice()).into_owned()
    }

    pub fn parse_unsigned_integer(&self) -> Result<u64, RedisError> {
        let val = self.parse_integer()?;
        u64::try_from(val)
            .map_err(|_| RedisError::Str("Couldn't parse negative number as unsigned integer"))
    }

    pub fn parse_integer(&self) -> Result<i64, RedisError> {
        let mut val: i64 = 0;
        match raw::string_to_longlong(self.inner, &mut val) {
            raw::Status::Ok => Ok(val),
            raw::Status::Err => Err(RedisError::Str("Couldn't parse as integer")),
        }
    }

    pub fn parse_float(&self) -> Result<f64, RedisError> {
        let mut val: f64 = 0.0;
        match raw::string_to_double(self.inner, &mut val) {
            raw::Status::Ok => Ok(val),
            raw::Status::Err => Err(RedisError::Str("Couldn't parse as float")),
        }
    }

    // TODO: Redis allows storing and retrieving any arbitrary bytes.
    // However rust's String and str can only store valid UTF-8.
    // Implement these to allow non-utf8 bytes to be consumed:
    // pub fn into_bytes(self) -> Vec<u8> {}
    // pub fn as_bytes(&self) -> &[u8] {}
}

impl Drop for RedisString {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe {
                raw::RedisModule_FreeString.unwrap()(self.ctx, self.inner);
            }
        }
    }
}

impl PartialEq for RedisString {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for RedisString {}

impl PartialOrd for RedisString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RedisString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        raw::string_compare(self.inner, other.inner)
    }
}

impl core::hash::Hash for RedisString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}

impl Display for RedisString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string_lossy())
    }
}

impl Borrow<str> for RedisString {
    fn borrow(&self) -> &str {
        // RedisString might not be UTF-8 safe
        self.try_as_str().unwrap_or("<Invalid UTF-8 data>")
    }
}

impl Clone for RedisString {
    fn clone(&self) -> Self {
        let inner =
            unsafe { raw::RedisModule_CreateStringFromString.unwrap()(self.ctx, self.inner) };
        Self::new(self.ctx, inner)
    }
}

impl From<RedisString> for String {
    fn from(rs: RedisString) -> Self {
        rs.to_string_lossy()
    }
}

///////////////////////////////////////////////////

#[derive(Debug)]
pub struct RedisBuffer {
    buffer: *mut c_char,
    len: usize,
}

impl RedisBuffer {
    pub fn new(buffer: *mut c_char, len: usize) -> Self {
        Self { buffer, len }
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
            raw::RedisModule_Free.unwrap()(self.buffer.cast::<c_void>());
        }
    }
}
