use std::borrow::Borrow;
use std::convert::TryFrom;
use std::ffi::CString;
use std::fmt::Display;
use std::ops::Deref;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr::NonNull;
use std::slice;
use std::str;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::{fmt, ptr};

use serde::de::{Error, SeqAccess};

pub use crate::raw;
pub use crate::rediserror::RedisError;
pub use crate::redisvalue::RedisValue;
use crate::Context;

/// A short-hand type that stores a [std::result::Result] with custom
/// type and [RedisError].
pub type RedisResult<T = RedisValue> = Result<T, RedisError>;
/// A [RedisResult] with [RedisValue].
pub type RedisValueResult = RedisResult<RedisValue>;

impl From<RedisValue> for RedisValueResult {
    fn from(v: RedisValue) -> Self {
        Ok(v)
    }
}

impl From<RedisError> for RedisValueResult {
    fn from(v: RedisError) -> Self {
        Err(v)
    }
}

pub const REDIS_OK: RedisValueResult = Ok(RedisValue::SimpleStringStatic("OK"));
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
    if argv.is_null() {
        return Vec::new();
    }
    unsafe { slice::from_raw_parts(argv, argc as usize) }
        .iter()
        .map(|&arg| RedisString::new(NonNull::new(ctx), arg))
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

    pub fn new(
        ctx: Option<NonNull<raw::RedisModuleCtx>>,
        inner: *mut raw::RedisModuleString,
    ) -> Self {
        let ctx = ctx.map_or(std::ptr::null_mut(), |v| v.as_ptr());
        raw::string_retain_string(ctx, inner);
        Self { ctx, inner }
    }

    /// In general, [RedisModuleString] is none atomic ref counted object.
    /// So it is not safe to clone it if Redis GIL is not held.
    /// [Self::safe_clone] gets a context reference which indicates that Redis GIL is held.
    pub fn safe_clone(&self, _ctx: &Context) -> Self {
        // RedisString are *not* atomic ref counted, so we must get a lock indicator to clone them.
        // Alos notice that Redis allows us to create RedisModuleString with NULL context
        // so we use [std::ptr::null_mut()] instead of the curren RedisString context.
        // We do this because we can not promise the new RedisString will not outlive the current
        // context and we want them to be independent.
        raw::string_retain_string(ptr::null_mut(), self.inner);
        Self {
            ctx: ptr::null_mut(),
            inner: self.inner,
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn create<T: Into<Vec<u8>>>(ctx: Option<NonNull<raw::RedisModuleCtx>>, s: T) -> Self {
        let ctx = ctx.map_or(std::ptr::null_mut(), |v| v.as_ptr());
        let str = CString::new(s).unwrap();
        let inner = unsafe {
            raw::RedisModule_CreateString.unwrap()(ctx, str.as_ptr(), str.as_bytes().len())
        };

        Self { ctx, inner }
    }

    /// Create a RedisString from a raw C string and length. The String will be copied.
    ///
    /// # Safety
    /// The caller must ensure that the provided pointer is valid and points to a memory region
    /// that is at least `len` bytes long.
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub unsafe fn from_raw_parts(
        ctx: Option<NonNull<raw::RedisModuleCtx>>,
        s: *const c_char,
        len: libc::size_t,
    ) -> Self {
        let ctx = ctx.map_or(std::ptr::null_mut(), |v| v.as_ptr());

        let inner = unsafe { raw::RedisModule_CreateString.unwrap()(ctx, s, len) };

        Self { ctx, inner }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn create_from_slice(ctx: *mut raw::RedisModuleCtx, s: &[u8]) -> Self {
        let inner = unsafe {
            raw::RedisModule_CreateString.unwrap()(ctx, s.as_ptr().cast::<c_char>(), s.len())
        };

        Self { ctx, inner }
    }

    pub const fn from_redis_module_string(
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

    #[must_use]
    pub fn as_cstr_ptr_and_len(&self) -> (*const c_char, usize) {
        let mut len: usize = 0;
        let ptr = raw::string_ptr_len(self.inner, &mut len);
        (ptr, len)
    }

    pub fn try_as_str<'a>(&self) -> Result<&'a str, RedisError> {
        Self::from_ptr(self.inner).map_err(|_| RedisError::Str("Couldn't parse as UTF-8 string"))
    }

    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        Self::string_as_slice(self.inner)
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn string_as_slice<'a>(ptr: *const raw::RedisModuleString) -> &'a [u8] {
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
            // Redis allows us to create RedisModuleString with NULL context
            // so we use [std::ptr::null_mut()] instead of the curren RedisString context.
            // We do this because we can not promise the new RedisString will not outlive the current
            // context and we want them to be independent.
            unsafe { raw::RedisModule_CreateStringFromString.unwrap()(ptr::null_mut(), self.inner) };
        Self::from_redis_module_string(ptr::null_mut(), inner)
    }
}

impl From<RedisString> for String {
    fn from(rs: RedisString) -> Self {
        rs.to_string_lossy()
    }
}

impl Deref for RedisString {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl From<RedisString> for Vec<u8> {
    fn from(rs: RedisString) -> Self {
        rs.as_slice().to_vec()
    }
}

impl serde::Serialize for RedisString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.as_slice())
    }
}

struct RedisStringVisitor;

impl<'de> serde::de::Visitor<'de> for RedisStringVisitor {
    type Value = RedisString;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A bytes buffer")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RedisString::create(None, v))
    }

    fn visit_seq<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut v = if let Some(size_hint) = visitor.size_hint() {
            Vec::with_capacity(size_hint)
        } else {
            Vec::new()
        };
        while let Some(elem) = visitor.next_element()? {
            v.push(elem);
        }

        Ok(RedisString::create(None, v.as_slice()))
    }
}

impl<'de> serde::Deserialize<'de> for RedisString {
    fn deserialize<D>(deserializer: D) -> Result<RedisString, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_bytes(RedisStringVisitor)
    }
}

///////////////////////////////////////////////////

#[derive(Debug)]
pub struct RedisBuffer {
    buffer: *mut c_char,
    len: usize,
}

impl RedisBuffer {
    pub const fn new(buffer: *mut c_char, len: usize) -> Self {
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
