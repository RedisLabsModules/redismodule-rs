use std::ffi::CString;
use std::os::raw::c_long;
use std::ptr;

use crate::key::{RedisKey, RedisKeyWritable};
use crate::raw;
use crate::LogLevel;
use crate::{RedisError, RedisResult, RedisString, RedisValue};

const FMT0: *const i8 = b"\0".as_ptr() as *const i8;
const FMT1: *const i8 = b"s\0".as_ptr() as *const i8;
const FMT2: *const i8 = b"ss\0".as_ptr() as *const i8;
const FMT3: *const i8 = b"sss\0".as_ptr() as *const i8;
const FMT4: *const i8 = b"ssss\0".as_ptr() as *const i8;
const FMT5: *const i8 = b"sssss\0".as_ptr() as *const i8;
const FMT6: *const i8 = b"ssssss\0".as_ptr() as *const i8;
const FMT7: *const i8 = b"sssssss\0".as_ptr() as *const i8;
const FMT8: *const i8 = b"ssssssss\0".as_ptr() as *const i8;
const FMT9: *const i8 = b"sssssssss\0".as_ptr() as *const i8;
const FMT10: *const i8 = b"ssssssssss\0".as_ptr() as *const i8;

/// Redis is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
pub struct Context {
    ctx: *mut raw::RedisModuleCtx,
}

impl Context {
    pub fn new(ctx: *mut raw::RedisModuleCtx) -> Self {
        Self { ctx }
    }

    pub fn dummy() -> Self {
        Self {
            ctx: ptr::null_mut(),
        }
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        let level = CString::new(format!("{:?}", level).to_lowercase()).unwrap();
        let fmt = CString::new(message).unwrap();
        unsafe { raw::RedisModule_Log.unwrap()(self.ctx, level.as_ptr(), fmt.as_ptr()) }
    }

    pub fn log_debug(&self, message: &str) {
        self.log(LogLevel::Notice, message);
    }

    pub fn auto_memory(&self) {
        unsafe {raw::RedisModule_AutoMemory.unwrap()(self.ctx);}
    }

    pub fn call(&self, cmd: &str, args: &[&str]) -> RedisResult {
        let args: Vec<RedisString> = args
            .iter()
            .map(|s| RedisString::create(self.ctx, s))
            .collect();
        let cmd = CString::new(cmd).unwrap();
        let reply: *mut raw::RedisModuleCallReply = unsafe {
            let p_call = raw::RedisModule_Call.unwrap();
            match args.len() {
                0 => p_call(self.ctx, cmd.as_ptr(), FMT0),
                1 => p_call(self.ctx, cmd.as_ptr(), FMT1, args[0].inner as *mut i8),
                2 => p_call(self.ctx, cmd.as_ptr(), FMT2, args[0].inner as *mut i8, args[1].inner as *mut i8),
                3 => p_call(self.ctx, cmd.as_ptr(), FMT3, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8),
                4 => p_call(self.ctx, cmd.as_ptr(), FMT4, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8),
                5 => p_call(self.ctx, cmd.as_ptr(), FMT5, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8),
                6 => p_call(self.ctx, cmd.as_ptr(), FMT6, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8, args[5].inner as *mut i8),
                7 => p_call(self.ctx, cmd.as_ptr(), FMT7, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8, args[5].inner as *mut i8, args[6].inner as *mut i8),
                8 => p_call(self.ctx, cmd.as_ptr(), FMT8, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8, args[5].inner as *mut i8, args[6].inner as *mut i8, args[7].inner as *mut i8),
                9 => p_call(self.ctx, cmd.as_ptr(), FMT9, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8, args[5].inner as *mut i8, args[6].inner as *mut i8, args[7].inner as *mut i8, args[8].inner as *mut i8),
                10 => p_call(self.ctx, cmd.as_ptr(), FMT10, args[0].inner as *mut i8, args[1].inner as *mut i8, args[2].inner as *mut i8, args[3].inner as *mut i8, args[4].inner as *mut i8, args[5].inner as *mut i8, args[6].inner as *mut i8, args[7].inner as *mut i8, args[8].inner as *mut i8, args[9].inner as *mut i8),
                _ => {return Err(RedisError::Str("too many arguments"));}
            }
        };

        let result = match raw::call_reply_type(reply) {
            raw::ReplyType::Unknown | raw::ReplyType::Error => {
                Err(RedisError::String(raw::call_reply_string(reply)))
            }
            _ => {
                Ok(RedisValue::SimpleStringStatic("OK"))
            }
        };
        raw::free_call_reply(reply);
        result
    }

    pub fn reply(&self, r: RedisResult) -> raw::Status {
        match r {
            Ok(RedisValue::Integer(v)) => unsafe {
                raw::RedisModule_ReplyWithLongLong.unwrap()(self.ctx, v).into()
            },

            Ok(RedisValue::Float(v)) => unsafe {
                raw::RedisModule_ReplyWithDouble.unwrap()(self.ctx, v).into()
            },

            Ok(RedisValue::SimpleStringStatic(s)) => unsafe {
                let msg = CString::new(s).unwrap();
                raw::RedisModule_ReplyWithSimpleString.unwrap()(self.ctx, msg.as_ptr()).into()
            },

            Ok(RedisValue::SimpleString(s)) => unsafe {
                let msg = CString::new(s).unwrap();
                raw::RedisModule_ReplyWithSimpleString.unwrap()(self.ctx, msg.as_ptr()).into()
            },

            Ok(RedisValue::BulkString(s)) => unsafe {
                raw::RedisModule_ReplyWithString.unwrap()(
                    self.ctx,
                    RedisString::create(self.ctx, s.as_ref()).inner,
                )
                .into()
            },

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

            Ok(RedisValue::None) => unsafe {
                raw::RedisModule_ReplyWithNull.unwrap()(self.ctx).into()
            },

            Err(RedisError::WrongArity) => unsafe {
                raw::RedisModule_WrongArity.unwrap()(self.ctx).into()
            },

            Err(RedisError::String(s)) => unsafe {
                let msg = CString::new(s).unwrap();
                raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into()
            },

            Err(RedisError::Str(s)) => unsafe {
                let msg = CString::new(s).unwrap();
                raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into()
            },
        }
    }

    pub fn open_key(&self, key: &str) -> RedisKey {
        RedisKey::open(self.ctx, key)
    }

    pub fn open_key_writable(&self, key: &str) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }

    pub fn replicate_verbatim(&self) {
        raw::replicate_verbatim(self.ctx);
    }
}
