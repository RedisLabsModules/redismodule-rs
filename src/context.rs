use std::os::raw::{c_long, c_longlong};
use std::ffi::CString;

use crate::raw;
use crate::error::Error;
use crate::{RedisString, LogLevel, Reply};
use crate::key::{RedisKey,RedisKeyWritable};

/// Redis is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
pub struct Context {
    ctx: *mut raw::RedisModuleCtx,
}

impl Context {
    pub fn new(ctx: *mut raw::RedisModuleCtx) -> Self {
        Self {ctx}
    }

    /// Coerces a Redis string as an integer.
    ///
    /// Redis is pretty dumb about data types. It nominally supports strings
    /// versus integers, but an integer set in the store will continue to look
    /// like a string (i.e. "1234") until some other operation like INCR forces
    /// its coercion.
    ///
    /// This method coerces a Redis string that looks like an integer into an
    /// integer response. All other types of replies are passed through
    /// unmodified.
    pub fn coerce_integer(
        &self,
        reply_res: Result<Reply, Error>,
    ) -> Result<Reply, Error> {
        match reply_res {
            Ok(Reply::String(s)) => match s.parse::<i64>() {
                Ok(n) => Ok(Reply::Integer(n)),
                _ => Ok(Reply::String(s)),
            },
            _ => reply_res,
        }
    }

    pub fn create_string(&self, s: &str) -> RedisString {
        RedisString::create(self.ctx, s)
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        let level = CString::new(format!("{:?}", level).to_lowercase()).unwrap();
        let fmt = CString::new(message).unwrap();
        raw::log(
            self.ctx,
            level.as_ptr(),
            fmt.as_ptr(),
        );
    }

    pub fn log_debug(&self, message: &str) {
        // Note that we log our debug messages as notice level in Redis. This
        // is so that they'll show up with default configuration. Our debug
        // logging will get compiled out in a release build so this won't
        // result in undue noise in production.
        self.log(LogLevel::Notice, message);
    }

    /// Opens a Redis key for read access.
    pub fn open_key(&self, key: &str) -> RedisKey {
        RedisKey::open(self.ctx, key)
    }

    /// Opens a Redis key for read and write access.
    pub fn open_key_writable(&self, key: &str) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }

    /// Tells Redis that we're about to reply with an (Redis) array.
    ///
    /// Used by invoking once with the expected length and then calling any
    /// combination of the other reply_* methods exactly that number of times.
    pub fn reply_array(&self, len: i64) -> Result<(), Error> {
        handle_status(
            raw::reply_with_array(self.ctx, len as c_long),
            "Could not reply with long",
        )
    }

    pub fn reply_integer(&self, integer: i64) -> Result<(), Error> {
        handle_status(
            raw::reply_with_long_long(self.ctx, integer as c_longlong),
            "Could not reply with longlong",
        )
    }

    pub fn reply_string(&self, message: &str) -> Result<(), Error> {
        let redis_str = self.create_string(message);
        handle_status(
            raw::reply_with_string(self.ctx, redis_str.str_inner),
            "Could not reply with string",
        )
    }
}

fn handle_status(status: raw::Status, message: &str) -> Result<(), Error> {
    match status {
        raw::Status::Ok => Ok(()),
        raw::Status::Err => Err(error!(message)),
    }
}
