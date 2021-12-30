use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long};
use std::ptr;

use crate::key::{RedisKey, RedisKeyWritable};
use crate::raw::{ModuleOptions, Version};
use crate::LogLevel;
use crate::{raw, utils};
use crate::{RedisError, RedisResult, RedisString, RedisValue};

#[cfg(feature = "experimental-api")]
mod timer;

#[cfg(feature = "experimental-api")]
pub(crate) mod thread_safe;

#[cfg(feature = "experimental-api")]
pub(crate) mod blocked;

pub(crate) mod info;

/// `Context` is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
pub struct Context {
    pub ctx: *mut raw::RedisModuleCtx,
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
        crate::logging::log_internal(self.ctx, level, message);
    }

    pub fn log_debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    pub fn log_notice(&self, message: &str) {
        self.log(LogLevel::Notice, message);
    }

    pub fn log_verbose(&self, message: &str) {
        self.log(LogLevel::Verbose, message);
    }

    pub fn log_warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_AutoMemory` is missing in redismodule.h
    pub fn auto_memory(&self) {
        unsafe {
            raw::RedisModule_AutoMemory.unwrap()(self.ctx);
        }
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_IsKeysPositionRequest` is missing in redismodule.h
    pub fn is_keys_position_request(&self) -> bool {
        // We want this to be available in tests where we don't have an actual Redis to call
        if cfg!(feature = "test") {
            return false;
        }

        let result = unsafe { raw::RedisModule_IsKeysPositionRequest.unwrap()(self.ctx) };

        result != 0
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_KeyAtPos` is missing in redismodule.h
    pub fn key_at_pos(&self, pos: i32) {
        // TODO: This will crash redis if `pos` is out of range.
        // Think of a way to make this safe by checking the range.
        unsafe {
            raw::RedisModule_KeyAtPos.unwrap()(self.ctx, pos as c_int);
        }
    }

    pub fn call(&self, command: &str, args: &[&str]) -> RedisResult {
        let terminated_args: Vec<RedisString> = args
            .iter()
            .map(|s| RedisString::create(self.ctx, s))
            .collect();

        let inner_args: Vec<*mut raw::RedisModuleString> =
            terminated_args.iter().map(|s| s.inner).collect();

        let cmd = CString::new(command).unwrap();
        let reply: *mut raw::RedisModuleCallReply = unsafe {
            let p_call = raw::RedisModule_Call.unwrap();
            p_call(
                self.ctx,
                cmd.as_ptr(),
                raw::FMT,
                inner_args.as_ptr() as *mut c_char,
                terminated_args.len(),
            )
        };
        let result = Self::parse_call_reply(reply);
        if !reply.is_null() {
            raw::free_call_reply(reply);
        }
        result
    }

    fn parse_call_reply(reply: *mut raw::RedisModuleCallReply) -> RedisResult {
        match raw::call_reply_type(reply) {
            raw::ReplyType::Error => Err(RedisError::String(raw::call_reply_string(reply))),
            raw::ReplyType::Unknown => Err(RedisError::Str("Error on method call")),
            raw::ReplyType::Array => {
                let length = raw::call_reply_length(reply);
                let mut vec = Vec::with_capacity(length);
                for i in 0..length {
                    vec.push(Self::parse_call_reply(raw::call_reply_array_element(
                        reply, i,
                    ))?)
                }
                Ok(RedisValue::Array(vec))
            }
            raw::ReplyType::Integer => Ok(RedisValue::Integer(raw::call_reply_integer(reply))),
            raw::ReplyType::String => Ok(RedisValue::SimpleString(raw::call_reply_string(reply))),
            raw::ReplyType::Null => Ok(RedisValue::Null),
        }
    }

    pub fn str_as_legal_resp_string(s: &str) -> CString {
        CString::new(
            s.chars()
                .map(|c| match c {
                    '\r' | '\n' => ' ' as u8,
                    _ => c as u8,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    pub fn reply_simple_string(&self, s: &str) -> raw::Status {
        let msg = Context::str_as_legal_resp_string(s);
        unsafe { raw::RedisModule_ReplyWithSimpleString.unwrap()(self.ctx, msg.as_ptr()).into() }
    }

    pub fn reply_error_string(&self, s: &str) -> raw::Status {
        let msg = Context::str_as_legal_resp_string(s);
        unsafe { raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into() }
    }

    /// # Panics
    ///
    /// Will panic if methods used are missing in redismodule.h
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
                raw::RedisModule_ReplyWithStringBuffer.unwrap()(
                    self.ctx,
                    s.as_ptr() as *const c_char,
                    s.len() as usize,
                )
                .into()
            },

            Ok(RedisValue::BulkRedisString(s)) => unsafe {
                raw::RedisModule_ReplyWithString.unwrap()(self.ctx, s.inner).into()
            },

            Ok(RedisValue::StringBuffer(s)) => unsafe {
                raw::RedisModule_ReplyWithStringBuffer.unwrap()(
                    self.ctx,
                    s.as_ptr() as *const c_char,
                    s.len() as usize,
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

            Ok(RedisValue::Null) => unsafe {
                raw::RedisModule_ReplyWithNull.unwrap()(self.ctx).into()
            },

            Ok(RedisValue::NoReply) => raw::Status::Ok,

            Err(RedisError::WrongArity) => unsafe {
                if self.is_keys_position_request() {
                    // We can't return a result since we don't have a client
                    raw::Status::Err
                } else {
                    raw::RedisModule_WrongArity.unwrap()(self.ctx).into()
                }
            },

            Err(RedisError::WrongType) => {
                self.reply_error_string(RedisError::WrongType.to_string().as_str())
            }

            Err(RedisError::String(s)) => self.reply_error_string(s.as_str()),

            Err(RedisError::Str(s)) => self.reply_error_string(s),
        }
    }

    pub fn open_key(&self, key: &RedisString) -> RedisKey {
        RedisKey::open(self.ctx, key)
    }

    pub fn open_key_writable(&self, key: &RedisString) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }

    pub fn replicate_verbatim(&self) {
        raw::replicate_verbatim(self.ctx);
    }

    pub fn create_string(&self, s: &str) -> RedisString {
        RedisString::create(self.ctx, s)
    }

    pub fn get_raw(&self) -> *mut raw::RedisModuleCtx {
        self.ctx
    }

    #[cfg(feature = "experimental-api")]
    pub fn export_shared_api(
        &self,
        func: *const ::std::os::raw::c_void,
        name: *const ::std::os::raw::c_char,
    ) {
        raw::export_shared_api(self.ctx, func, name);
    }

    #[cfg(feature = "experimental-api")]
    pub fn notify_keyspace_event(
        &self,
        event_type: raw::NotifyEvent,
        event: &str,
        keyname: &RedisString,
    ) -> raw::Status {
        raw::notify_keyspace_event(self.ctx, event_type, event, keyname)
    }

    /// Returns the redis version either by calling RedisModule_GetServerVersion API,
    /// Or if it is not available, by calling "info server" API and parsing the reply
    pub fn get_redis_version(&self) -> Result<Version, RedisError> {
        self.get_redis_version_internal(false)
    }

    /// Returns the redis version by calling "info server" API and parsing the reply
    #[cfg(feature = "test")]
    pub fn get_redis_version_rm_call(&self) -> Result<Version, RedisError> {
        self.get_redis_version_internal(true)
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn get_redis_version_internal(&self, force_use_rm_call: bool) -> Result<Version, RedisError> {
        match unsafe { raw::RedisModule_GetServerVersion } {
            Some(api) if !force_use_rm_call => {
                // Call existing API
                Ok(Version::from(unsafe { api() }))
            }
            _ => {
                // Call "info server"
                if let Ok(RedisValue::SimpleString(s)) = self.call("info", &["server"]) {
                    if let Some(ver) = utils::get_regexp_captures(
                        s.as_str(),
                        r"(?m)\bredis_version:([0-9]+)\.([0-9]+)\.([0-9]+)\b",
                    ) {
                        return Ok(Version {
                            major: ver[0].parse::<c_int>().unwrap(),
                            minor: ver[1].parse::<c_int>().unwrap(),
                            patch: ver[2].parse::<c_int>().unwrap(),
                        });
                    }
                }
                Err(RedisError::Str(
                    "Error getting redis_version from \"info server\" call",
                ))
            }
        }
    }
    pub fn set_module_options(&self, options: ModuleOptions) {
        unsafe { raw::RedisModule_SetModuleOptions.unwrap()(self.ctx, options.bits()) };
    }
}
