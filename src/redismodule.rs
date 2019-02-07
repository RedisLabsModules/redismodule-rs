use std::slice;
use std::str;
use std::ffi::CString;

pub type RedisResult = Result<RedisValue, RedisError>;

use crate::raw;

#[derive(Debug)]
pub enum RedisError {
    WrongArity,
    String(&'static str),
}

#[derive(Debug)]
pub enum RedisValue {
    String(String),
    Integer(i64),
    Array(Vec<RedisValue>),
}

#[derive(Debug)]
pub struct RedisString {
    ctx: *mut raw::RedisModuleCtx,
    pub inner: *mut raw::RedisModuleString,
}

impl RedisString {
    pub fn create(ctx: *mut raw::RedisModuleCtx, s: &str) -> RedisString {
        let str = CString::new(s).unwrap();
        let inner = unsafe {
            raw::RedisModule_CreateString.unwrap()(ctx, str.as_ptr(), s.len())
        };

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

