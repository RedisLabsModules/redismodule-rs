use std::os::raw::c_long;
use std::ffi::CString;

use crate::raw;
use crate::error::Error;
use crate::LogLevel;
use crate::key::{RedisKey, RedisKeyWritable};
use crate::{RedisString, RedisError, RedisValue, RedisResult};

/// Redis is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
pub struct Context {
    ctx: *mut raw::RedisModuleCtx,
}

impl Context {
    pub fn new(ctx: *mut raw::RedisModuleCtx) -> Self {
        Self { ctx }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        let level = CString::new(format!("{:?}", level).to_lowercase()).unwrap();
        let fmt = CString::new(message).unwrap();
        unsafe { raw::RedisModule_Log.unwrap()(self.ctx, level.as_ptr(), fmt.as_ptr()) }
    }

    pub fn log_debug(&self, message: &str) {
        self.log(LogLevel::Notice, message);
    }

    pub fn call(&self, command: &str, args: &[&str]) -> RedisResult {
        let terminated_args: Vec<RedisString> = args.iter()
            .map(|s| RedisString::create(self.ctx, s))
            .collect();

        let _ = terminated_args;
        let _ = command;
        let _ = args;

        return Err(RedisError::String("not implemented"));
    }

    pub fn reply(&self, r: RedisResult) -> raw::Status {
        match r {
            Ok(RedisValue::Integer(v)) => unsafe {
                raw::RedisModule_ReplyWithLongLong.unwrap()(self.ctx, v).into()
            }

            Ok(RedisValue::String(s)) => unsafe {
                raw::RedisModule_ReplyWithString.unwrap()(
                    self.ctx,
                    RedisString::create(self.ctx, s.as_ref()).inner).into()
            }

            Ok(RedisValue::Array(array)) => {
                unsafe {
                    // According to the Redis source code this always succeeds,
                    // so there is no point in checking its return value.
                    raw::RedisModule_ReplyWithArray.unwrap()(self.ctx, array.len() as c_long);
                }

                for elem in array {
                    self.reply(Ok(elem));
                }

                raw::Status::Ok
            }

            Err(RedisError::WrongArity) => unsafe {
                raw::RedisModule_WrongArity.unwrap()(self.ctx).into()
            }

            Err(RedisError::String(s)) => unsafe {
                let msg = CString::new(s).unwrap();
                raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into()
            }
        }
    }

    pub fn open_key(&self, key: &str) -> RedisKey {
        RedisKey::open(self.ctx, key)
    }

    pub fn open_key_writable(&self, key: &str) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }
}

fn handle_status(status: raw::Status, message: &str) -> Result<(), Error> {
    match status {
        raw::Status::Ok => Ok(()),
        raw::Status::Err => Err(error!(message)),
    }
}
