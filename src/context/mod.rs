use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_longlong};
use std::ptr;

use crate::key::{RedisKey, RedisKeyWritable};
use crate::raw::{ModuleOptions, Version};
use crate::{add_info_field_long_long, add_info_field_str, raw, utils, Status};
use crate::{add_info_section, LogLevel};
use crate::{RedisError, RedisResult, RedisString, RedisValue};

#[cfg(feature = "experimental-api")]
use std::ffi::CStr;

#[cfg(feature = "experimental-api")]
mod timer;

#[cfg(feature = "experimental-api")]
pub mod thread_safe;

#[cfg(feature = "experimental-api")]
pub mod blocked;

pub mod info;

pub mod keys_cursor;

pub mod server_events;

pub mod configuration;

#[derive(Clone)]
pub struct CallOptions {
    options: String,
}

pub struct CallOptionsBuilder {
    options: String,
}

impl CallOptionsBuilder {
    pub fn new() -> CallOptionsBuilder {
        CallOptionsBuilder {
            options: "v".to_string(),
        }
    }

    fn add_flag(&mut self, flag: &str) {
        self.options.push_str(flag);
    }

    pub fn no_writes(mut self) -> CallOptionsBuilder {
        self.add_flag("W");
        self
    }

    pub fn script_mode(mut self) -> CallOptionsBuilder {
        self.add_flag("S");
        self
    }

    pub fn verify_acl(mut self) -> CallOptionsBuilder {
        self.add_flag("C");
        self
    }

    pub fn verify_oom(mut self) -> CallOptionsBuilder {
        self.add_flag("M");
        self
    }

    pub fn errors_as_replies(mut self) -> CallOptionsBuilder {
        self.add_flag("E");
        self
    }

    pub fn replicate(mut self) -> CallOptionsBuilder {
        self.add_flag("!");
        self
    }

    pub fn constract(&self) -> CallOptions {
        let mut res = CallOptions {
            options: self.options.to_string(),
        };
        res.options.push_str("\0"); /* make it C string */
        res
    }
}

pub struct AclPermissions {
    flags: u32,
}

impl AclPermissions {
    pub fn new() -> AclPermissions {
        AclPermissions { flags: 0 }
    }

    pub fn add_access_permission(&mut self) {
        self.flags |= raw::REDISMODULE_CMD_KEY_ACCESS;
    }

    pub fn add_insert_permission(&mut self) {
        self.flags |= raw::REDISMODULE_CMD_KEY_INSERT;
    }

    pub fn add_delete_permission(&mut self) {
        self.flags |= raw::REDISMODULE_CMD_KEY_DELETE;
    }

    pub fn add_update_permission(&mut self) {
        self.flags |= raw::REDISMODULE_CMD_KEY_UPDATE;
    }

    pub fn add_full_permission(&mut self) {
        self.add_access_permission();
        self.add_insert_permission();
        self.add_delete_permission();
        self.add_update_permission();
    }
}

/// `Context` is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
pub struct Context {
    pub ctx: *mut raw::RedisModuleCtx,
}

impl Context {
    pub const fn new(ctx: *mut raw::RedisModuleCtx) -> Self {
        Self { ctx }
    }

    #[must_use]
    pub const fn dummy() -> Self {
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
    #[must_use]
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

    pub fn call_internal(
        &self,
        command: &str,
        options: *const c_char,
        args: &[&str],
    ) -> RedisResult {
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
                options,
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

    pub fn call_ext(&self, command: &str, options: &CallOptions, args: &[&str]) -> RedisResult {
        self.call_internal(command, options.options.as_ptr() as *const c_char, args)
    }

    pub fn call(&self, command: &str, args: &[&str]) -> RedisResult {
        self.call_internal(command, raw::FMT, args)
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
                    ))?);
                }
                Ok(RedisValue::Array(vec))
            }
            raw::ReplyType::Integer => Ok(RedisValue::Integer(raw::call_reply_integer(reply))),
            raw::ReplyType::String => Ok(RedisValue::SimpleString(raw::call_reply_string(reply))),
            raw::ReplyType::Null => Ok(RedisValue::Null),
        }
    }

    #[must_use]
    pub fn str_as_legal_resp_string(s: &str) -> CString {
        CString::new(
            s.chars()
                .map(|c| match c {
                    '\r' | '\n' | '\0' => b' ',
                    _ => c as u8,
                })
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_simple_string(&self, s: &str) -> raw::Status {
        let msg = Self::str_as_legal_resp_string(s);
        unsafe { raw::RedisModule_ReplyWithSimpleString.unwrap()(self.ctx, msg.as_ptr()).into() }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_bulk_string(&self, s: &str) -> raw::Status {
        unsafe {
            raw::RedisModule_ReplyWithStringBuffer.unwrap()(
                self.ctx,
                s.as_ptr() as *mut c_char,
                s.len(),
            )
            .into()
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_array(&self, size: usize) -> raw::Status {
        unsafe { raw::RedisModule_ReplyWithArray.unwrap()(self.ctx, size as c_long).into() }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_long(&self, l: i64) -> raw::Status {
        unsafe { raw::RedisModule_ReplyWithLongLong.unwrap()(self.ctx, l as c_longlong).into() }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_double(&self, d: f64) -> raw::Status {
        unsafe { raw::RedisModule_ReplyWithDouble.unwrap()(self.ctx, d).into() }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_error_string(&self, s: &str) -> raw::Status {
        let msg = Self::str_as_legal_resp_string(s);
        unsafe { raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into() }
    }

    /// # Panics
    ///
    /// Will panic if methods used are missing in redismodule.h
    #[allow(clippy::must_use_candidate)]
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
                    s.as_ptr().cast::<c_char>(),
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
                    s.as_ptr().cast::<c_char>(),
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

    #[must_use]
    pub fn open_key(&self, key: &RedisString) -> RedisKey {
        RedisKey::open(self.ctx, key)
    }

    #[must_use]
    pub fn open_key_writable(&self, key: &RedisString) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }

    pub fn replicate_verbatim(&self) {
        raw::replicate_verbatim(self.ctx);
    }

    #[must_use]
    pub fn create_string(&self, s: &str) -> RedisString {
        RedisString::create(self.ctx, s)
    }

    #[must_use]
    pub const fn get_raw(&self) -> *mut raw::RedisModuleCtx {
        self.ctx
    }

    /// # Safety
    #[cfg(feature = "experimental-api")]
    pub unsafe fn export_shared_api(
        &self,
        func: *const ::std::os::raw::c_void,
        name: *const ::std::os::raw::c_char,
    ) {
        raw::export_shared_api(self.ctx, func, name);
    }

    #[cfg(feature = "experimental-api")]
    #[allow(clippy::must_use_candidate)]
    pub fn notify_keyspace_event(
        &self,
        event_type: raw::NotifyEvent,
        event: &str,
        keyname: &RedisString,
    ) -> raw::Status {
        unsafe { raw::notify_keyspace_event(self.ctx, event_type, event, keyname) }
    }

    #[cfg(feature = "experimental-api")]
    pub fn current_command_name(&self) -> Result<String, RedisError> {
        unsafe {
            match raw::RedisModule_GetCurrentCommandName {
                Some(cmd) => Ok(CStr::from_ptr(cmd(self.ctx)).to_str().unwrap().to_string()),
                None => Err(RedisError::Str(
                    "API RedisModule_GetCurrentCommandName is not available",
                )),
            }
        }
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

    pub fn version_from_info(info: RedisValue) -> Result<Version, RedisError> {
        if let RedisValue::SimpleString(info_str) = info {
            if let Some(ver) = utils::get_regexp_captures(
                info_str.as_str(),
                r"(?m)\bredis_version:([0-9]+)\.([0-9]+)\.([0-9]+)\b",
            ) {
                return Ok(Version {
                    major: ver[0].parse::<c_int>().unwrap(),
                    minor: ver[1].parse::<c_int>().unwrap(),
                    patch: ver[2].parse::<c_int>().unwrap(),
                });
            }
        }
        Err(RedisError::Str("Error getting redis_version"))
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
                if let Ok(info) = self.call("info", &["server"]) {
                    Context::version_from_info(info)
                } else {
                    Err(RedisError::Str("Error calling \"info server\""))
                }
            }
        }
    }

    pub fn set_module_options(&self, options: ModuleOptions) {
        unsafe { raw::RedisModule_SetModuleOptions.unwrap()(self.ctx, options.bits()) };
    }

    pub fn is_primary(&self) -> bool {
        let flags = unsafe { raw::RedisModule_GetContextFlags.unwrap()(self.ctx) };
        flags as u32 & raw::REDISMODULE_CTX_FLAGS_MASTER != 0
    }

    pub fn is_oom(&self) -> bool {
        let flags = unsafe { raw::RedisModule_GetContextFlags.unwrap()(self.ctx) };
        flags as u32 & raw::REDISMODULE_CTX_FLAGS_OOM != 0
    }

    pub fn get_current_user(&self) -> Result<String, RedisError> {
        let user = unsafe { raw::RedisModule_GetCurrentUserName.unwrap()(self.ctx) };
        let user = RedisString::from_redis_module_string(ptr::null_mut(), user);
        Ok(user.try_as_str()?.to_string())
    }

    pub fn autenticate_user(&self, user_name: &str) -> raw::Status {
        if unsafe {
            raw::RedisModule_AuthenticateClientWithACLUser.unwrap()(
                self.ctx,
                user_name.as_ptr() as *const c_char,
                user_name.len(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        } == raw::REDISMODULE_OK as i32
        {
            raw::Status::Ok
        } else {
            raw::Status::Err
        }
    }

    pub fn acl_check_key_permission(
        &self,
        user_name: &str,
        key_name: &RedisString,
        permissions: &AclPermissions,
    ) -> Result<(), RedisError> {
        let user_name = RedisString::create(self.ctx, user_name);
        let user = unsafe { raw::RedisModule_GetModuleUserFromUserName.unwrap()(user_name.inner) };
        if user.is_null() {
            return Err(RedisError::Str("User does not exists or disabled"));
        }
        if unsafe {
            raw::RedisModule_ACLCheckKeyPermissions.unwrap()(
                user,
                key_name.inner,
                permissions.flags as i32,
            )
        } == raw::REDISMODULE_OK as i32
        {
            unsafe { raw::RedisModule_FreeModuleUser.unwrap()(user) };
            Ok(())
        } else {
            unsafe { raw::RedisModule_FreeModuleUser.unwrap()(user) };
            Err(RedisError::Str("User does not have permissions on key"))
        }
    }
}

pub struct InfoContext {
    pub ctx: *mut raw::RedisModuleInfoCtx,
}

impl InfoContext {
    pub const fn new(ctx: *mut raw::RedisModuleInfoCtx) -> Self {
        Self { ctx }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn add_info_section(&self, name: Option<&str>) -> Status {
        add_info_section(self.ctx, name)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn add_info_field_str(&self, name: &str, content: &str) -> Status {
        add_info_field_str(self.ctx, name, content)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn add_info_field_long_long(&self, name: &str, value: c_longlong) -> Status {
        add_info_field_long_long(self.ctx, name, value)
    }
}
