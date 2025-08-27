use bitflags::bitflags;
use redis_module_macros_internals::api;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;
use std::os::raw::c_void;
use std::os::raw::{c_char, c_int, c_long, c_longlong};
use std::ptr::{self, NonNull};
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::key::{KeyFlags, RedisKey, RedisKeyWritable};
use crate::logging::RedisLogLevel;
use crate::raw::{ModuleOptions, Version};
use crate::redisvalue::RedisValueKey;
use crate::{
    add_info_begin_dict_field, add_info_end_dict_field, add_info_field_double,
    add_info_field_long_long, add_info_field_str, add_info_field_unsigned_long_long, raw, utils,
    Status,
};
use crate::{add_info_section, RedisResult};
use crate::{RedisError, RedisString, RedisValue};
use std::ops::Deref;

use std::ffi::CStr;

use self::call_reply::{create_promise_call_reply, CallResult, PromiseCallReply};
use self::thread_safe::RedisLockIndicator;

mod timer;

pub mod blocked;
pub mod call_reply;
pub mod commands;
pub mod defrag;
pub mod info;
pub mod key_scan_cursor;
pub mod keys_cursor;
pub mod server_events;
pub mod thread_safe;

pub struct CallOptionsBuilder {
    options: String,
}

impl Default for CallOptionsBuilder {
    fn default() -> Self {
        CallOptionsBuilder {
            options: "v".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct CallOptions {
    options: CString,
}

#[derive(Clone)]
#[cfg(any(
    feature = "min-redis-compatibility-version-7-4",
    feature = "min-redis-compatibility-version-7-2"
))]
pub struct BlockingCallOptions {
    options: CString,
}

#[derive(Copy, Clone)]
pub enum CallOptionResp {
    Resp2,
    Resp3,
    Auto,
}

impl CallOptionsBuilder {
    pub fn new() -> CallOptionsBuilder {
        Self::default()
    }

    fn add_flag(&mut self, flag: &str) {
        self.options.push_str(flag);
    }

    /// Enable this option will not allow RM_Call to perform write commands
    pub fn no_writes(mut self) -> CallOptionsBuilder {
        self.add_flag("W");
        self
    }

    /// Enable this option will run RM_Call is script mode.
    /// This mean that Redis will enable the following protections:
    /// 1. Not allow running dangerous commands like 'shutdown'
    /// 2. Not allow running write commands on OOM or if there are not enough good replica's connected
    pub fn script_mode(mut self) -> CallOptionsBuilder {
        self.add_flag("S");
        self
    }

    /// Enable this option will perform ACL validation on the user attached to the context that
    /// is used to invoke the call.
    pub fn verify_acl(mut self) -> CallOptionsBuilder {
        self.add_flag("C");
        self
    }

    /// Enable this option will OOM validation before running the command
    pub fn verify_oom(mut self) -> CallOptionsBuilder {
        self.add_flag("M");
        self
    }

    /// Enable this option will return error as CallReply object instead of setting errno (it is
    /// usually recommend to enable it)
    pub fn errors_as_replies(mut self) -> CallOptionsBuilder {
        self.add_flag("E");
        self
    }

    /// Enable this option will cause the command to be replicaed to the replica and AOF
    pub fn replicate(mut self) -> CallOptionsBuilder {
        self.add_flag("!");
        self
    }

    /// Allow control the protocol version in which the replies will be returned.
    pub fn resp(mut self, resp: CallOptionResp) -> CallOptionsBuilder {
        match resp {
            CallOptionResp::Auto => self.add_flag("0"),
            CallOptionResp::Resp2 => (),
            CallOptionResp::Resp3 => self.add_flag("3"),
        }
        self
    }

    /// Construct a CallOption object that can be used to run commands using call_ext
    pub fn build(self) -> CallOptions {
        CallOptions {
            options: CString::new(self.options).unwrap(), // the data will never contains internal \0 so it is safe to unwrap.
        }
    }

    /// Construct a CallOption object that can be used to run commands using call_blocking.
    /// The commands can be either blocking or none blocking. In case the command are blocking
    /// (like `blpop`) a [FutureCallReply] will be returned.
    #[cfg(any(
        feature = "min-redis-compatibility-version-7-4",
        feature = "min-redis-compatibility-version-7-2"
    ))]
    pub fn build_blocking(mut self) -> BlockingCallOptions {
        self.add_flag("K");
        BlockingCallOptions {
            options: CString::new(self.options).unwrap(), // the data will never contains internal \0 so it is safe to unwrap.
        }
    }
}

/// This struct allows logging when the Redis GIL is not acquired.
/// It is implemented `Send` and `Sync` so it can safely be used
/// from within different threads.
pub struct DetachedContext {
    pub(crate) ctx: AtomicPtr<raw::RedisModuleCtx>,
}

impl DetachedContext {
    pub const fn new() -> Self {
        DetachedContext {
            ctx: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl Default for DetachedContext {
    fn default() -> Self {
        Self::new()
    }
}

/// This object is returned after locking Redis from [DetachedContext].
/// On dispose, Redis will be unlocked.
/// This object implements [Deref] for [Context] so it can be used
/// just like any Redis [Context] for command invocation.
/// **This object should not be used to return replies** because there is
/// no real client behind this context to return replies to.
pub struct DetachedContextGuard {
    pub(crate) ctx: Context,
}

unsafe impl RedisLockIndicator for DetachedContextGuard {}

impl Drop for DetachedContextGuard {
    fn drop(&mut self) {
        unsafe {
            raw::RedisModule_ThreadSafeContextUnlock.unwrap()(self.ctx.ctx);
        };
    }
}

impl Deref for DetachedContextGuard {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl DetachedContext {
    pub fn log(&self, level: RedisLogLevel, message: &str) {
        let c = self.ctx.load(Ordering::Relaxed);
        crate::logging::log_internal(c, level, message);
    }

    pub fn log_debug(&self, message: &str) {
        self.log(RedisLogLevel::Debug, message);
    }

    pub fn log_notice(&self, message: &str) {
        self.log(RedisLogLevel::Notice, message);
    }

    pub fn log_verbose(&self, message: &str) {
        self.log(RedisLogLevel::Verbose, message);
    }

    pub fn log_warning(&self, message: &str) {
        self.log(RedisLogLevel::Warning, message);
    }

    pub fn set_context(&self, ctx: &Context) -> Result<(), RedisError> {
        let c = self.ctx.load(Ordering::Relaxed);
        if !c.is_null() {
            return Err(RedisError::Str("Detached context is already set"));
        }
        let ctx = unsafe { raw::RedisModule_GetDetachedThreadSafeContext.unwrap()(ctx.ctx) };
        self.ctx.store(ctx, Ordering::Relaxed);
        Ok(())
    }

    /// Lock Redis for command invocation. Returns [DetachedContextGuard] which will unlock Redis when dispose.
    /// [DetachedContextGuard] implements [Deref<Target = Context>] so it can be used just like any Redis [Context] for command invocation.
    /// Locking Redis when Redis is already locked by the current thread is left unspecified.
    /// However, this function will not return on the second call (it might panic or deadlock, for example)..
    pub fn lock(&self) -> DetachedContextGuard {
        let c = self.ctx.load(Ordering::Relaxed);
        unsafe { raw::RedisModule_ThreadSafeContextLock.unwrap()(c) };
        let ctx = Context::new(c);
        DetachedContextGuard { ctx }
    }
}

unsafe impl Send for DetachedContext {}
unsafe impl Sync for DetachedContext {}

/// `Context` is a structure that's designed to give us a high-level interface to
/// the Redis module API by abstracting away the raw C FFI calls.
#[derive(Debug)]
pub struct Context {
    pub ctx: *mut raw::RedisModuleCtx,
}

/// A guerd that protected a user that has
/// been set on a context using `autenticate_user`.
/// This guerd make sure to unset the user when freed.
/// It prevent privilege escalation security issues
/// that can happened by forgeting to unset the user.
#[derive(Debug)]
pub struct ContextUserScope<'ctx> {
    ctx: &'ctx Context,
    user: *mut raw::RedisModuleUser,
}

impl<'ctx> Drop for ContextUserScope<'ctx> {
    fn drop(&mut self) {
        self.ctx.deautenticate_user();
        unsafe { raw::RedisModule_FreeModuleUser.unwrap()(self.user) };
    }
}

impl<'ctx> ContextUserScope<'ctx> {
    fn new(ctx: &'ctx Context, user: *mut raw::RedisModuleUser) -> ContextUserScope<'ctx> {
        ContextUserScope { ctx, user }
    }
}

pub struct StrCallArgs<'a> {
    is_owner: bool,
    args: Vec<*mut raw::RedisModuleString>,
    // Phantom is used to make sure the object will not live longer than actual arguments slice
    phantom: std::marker::PhantomData<&'a raw::RedisModuleString>,
}

impl<'a> Drop for StrCallArgs<'a> {
    fn drop(&mut self) {
        if self.is_owner {
            self.args.iter_mut().for_each(|v| unsafe {
                raw::RedisModule_FreeString.unwrap()(std::ptr::null_mut(), *v)
            });
        }
    }
}

impl<'a, T: AsRef<[u8]> + ?Sized> From<&'a [&T]> for StrCallArgs<'a> {
    fn from(vals: &'a [&T]) -> Self {
        StrCallArgs {
            is_owner: true,
            args: vals
                .iter()
                .map(|v| RedisString::create_from_slice(std::ptr::null_mut(), v.as_ref()).take())
                .collect(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<'a> From<&'a [&RedisString]> for StrCallArgs<'a> {
    fn from(vals: &'a [&RedisString]) -> Self {
        StrCallArgs {
            is_owner: false,
            args: vals.iter().map(|v| v.inner).collect(),
            phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, const SIZE: usize, T: ?Sized> From<&'a [&T; SIZE]> for StrCallArgs<'a>
where
    for<'b> &'a [&'b T]: Into<StrCallArgs<'a>>,
{
    fn from(vals: &'a [&T; SIZE]) -> Self {
        vals.as_ref().into()
    }
}

impl<'a> StrCallArgs<'a> {
    pub(crate) fn args_mut(&mut self) -> &mut [*mut raw::RedisModuleString] {
        &mut self.args
    }
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

    pub fn log(&self, level: RedisLogLevel, message: &str) {
        crate::logging::log_internal(self.ctx, level, message);
    }

    pub fn log_debug(&self, message: &str) {
        self.log(RedisLogLevel::Debug, message);
    }

    pub fn log_notice(&self, message: &str) {
        self.log(RedisLogLevel::Notice, message);
    }

    pub fn log_verbose(&self, message: &str) {
        self.log(RedisLogLevel::Verbose, message);
    }

    pub fn log_warning(&self, message: &str) {
        self.log(RedisLogLevel::Warning, message);
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
        if cfg!(test) {
            return false;
        }

        (unsafe { raw::RedisModule_IsKeysPositionRequest.unwrap()(self.ctx) }) != 0
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

    fn call_internal<
        'ctx,
        'a,
        T: Into<StrCallArgs<'a>>,
        R: From<PromiseCallReply<'static, 'ctx>>,
    >(
        &'ctx self,
        command: &str,
        fmt: *const c_char,
        args: T,
    ) -> R {
        let mut call_args: StrCallArgs = args.into();
        let final_args = call_args.args_mut();

        let cmd = CString::new(command).unwrap();
        let reply: *mut raw::RedisModuleCallReply = unsafe {
            let p_call = raw::RedisModule_Call.unwrap();
            p_call(
                self.ctx,
                cmd.as_ptr(),
                fmt,
                final_args.as_mut_ptr(),
                final_args.len(),
            )
        };
        let promise = create_promise_call_reply(self, NonNull::new(reply));
        R::from(promise)
    }

    pub fn call<'a, T: Into<StrCallArgs<'a>>>(&self, command: &str, args: T) -> RedisResult {
        self.call_internal::<_, CallResult>(command, raw::FMT, args)
            .map_or_else(|e| Err(e.into()), |v| Ok((&v).into()))
    }

    /// Invoke a command on Redis and return the result
    /// Unlike 'call' this API also allow to pass a CallOption to control different aspects
    /// of the command invocation.
    pub fn call_ext<'a, T: Into<StrCallArgs<'a>>, R: From<CallResult<'static>>>(
        &self,
        command: &str,
        options: &CallOptions,
        args: T,
    ) -> R {
        let res: CallResult<'static> =
            self.call_internal(command, options.options.as_ptr() as *const c_char, args);
        R::from(res)
    }

    /// Same as [call_ext] but also allow to perform blocking commands like BLPOP.
    #[cfg(any(
        feature = "min-redis-compatibility-version-7-4",
        feature = "min-redis-compatibility-version-7-2"
    ))]
    pub fn call_blocking<
        'ctx,
        'a,
        T: Into<StrCallArgs<'a>>,
        R: From<PromiseCallReply<'static, 'ctx>>,
    >(
        &'ctx self,
        command: &str,
        options: &BlockingCallOptions,
        args: T,
    ) -> R {
        self.call_internal(command, options.options.as_ptr() as *const c_char, args)
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
        raw::reply_with_simple_string(self.ctx, msg.as_ptr())
    }

    #[allow(clippy::must_use_candidate)]
    pub fn reply_error_string(&self, s: &str) -> raw::Status {
        let msg = Self::str_as_legal_resp_string(s);
        unsafe { raw::RedisModule_ReplyWithError.unwrap()(self.ctx, msg.as_ptr()).into() }
    }

    pub fn reply_with_key(&self, result: RedisValueKey) -> raw::Status {
        match result {
            RedisValueKey::Integer(i) => raw::reply_with_long_long(self.ctx, i),
            RedisValueKey::String(s) => {
                raw::reply_with_string_buffer(self.ctx, s.as_ptr().cast::<c_char>(), s.len())
            }
            RedisValueKey::BulkString(b) => {
                raw::reply_with_string_buffer(self.ctx, b.as_ptr().cast::<c_char>(), b.len())
            }
            RedisValueKey::BulkRedisString(s) => raw::reply_with_string(self.ctx, s.inner),
            RedisValueKey::Bool(b) => raw::reply_with_bool(self.ctx, b.into()),
        }
    }

    /// # Panics
    ///
    /// Will panic if methods used are missing in redismodule.h
    #[allow(clippy::must_use_candidate)]
    pub fn reply(&self, result: RedisResult) -> raw::Status {
        match result {
            Ok(RedisValue::Bool(v)) => raw::reply_with_bool(self.ctx, v.into()),
            Ok(RedisValue::Integer(v)) => raw::reply_with_long_long(self.ctx, v),
            Ok(RedisValue::Float(v)) => raw::reply_with_double(self.ctx, v),
            Ok(RedisValue::SimpleStringStatic(s)) => {
                let msg = CString::new(s).unwrap();
                raw::reply_with_simple_string(self.ctx, msg.as_ptr())
            }

            Ok(RedisValue::SimpleString(s)) => {
                let msg = CString::new(s).unwrap();
                raw::reply_with_simple_string(self.ctx, msg.as_ptr())
            }

            Ok(RedisValue::BulkString(s)) => {
                raw::reply_with_string_buffer(self.ctx, s.as_ptr().cast::<c_char>(), s.len())
            }

            Ok(RedisValue::BigNumber(s)) => {
                raw::reply_with_big_number(self.ctx, s.as_ptr().cast::<c_char>(), s.len())
            }

            Ok(RedisValue::VerbatimString((format, data))) => raw::reply_with_verbatim_string(
                self.ctx,
                data.as_ptr().cast(),
                data.len(),
                format.0.as_ptr().cast(),
            ),

            Ok(RedisValue::BulkRedisString(s)) => raw::reply_with_string(self.ctx, s.inner),

            Ok(RedisValue::StringBuffer(s)) => {
                raw::reply_with_string_buffer(self.ctx, s.as_ptr().cast::<c_char>(), s.len())
            }

            Ok(RedisValue::Array(array)) => {
                raw::reply_with_array(self.ctx, array.len() as c_long);

                for elem in array {
                    self.reply(Ok(elem));
                }

                raw::Status::Ok
            }

            Ok(RedisValue::Map(map)) => {
                raw::reply_with_map(self.ctx, map.len() as c_long);

                for (key, value) in map {
                    self.reply_with_key(key);
                    self.reply(Ok(value));
                }

                raw::Status::Ok
            }

            Ok(RedisValue::OrderedMap(map)) => {
                raw::reply_with_map(self.ctx, map.len() as c_long);

                for (key, value) in map {
                    self.reply_with_key(key);
                    self.reply(Ok(value));
                }

                raw::Status::Ok
            }

            Ok(RedisValue::Set(set)) => {
                raw::reply_with_set(self.ctx, set.len() as c_long);
                set.into_iter().for_each(|e| {
                    self.reply_with_key(e);
                });

                raw::Status::Ok
            }

            Ok(RedisValue::OrderedSet(set)) => {
                raw::reply_with_set(self.ctx, set.len() as c_long);
                set.into_iter().for_each(|e| {
                    self.reply_with_key(e);
                });

                raw::Status::Ok
            }

            Ok(RedisValue::Null) => raw::reply_with_null(self.ctx),

            Ok(RedisValue::NoReply) => raw::Status::Ok,

            Ok(RedisValue::StaticError(s)) => self.reply_error_string(s),

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
    pub fn open_key_with_flags(&self, key: &RedisString, flags: KeyFlags) -> RedisKey {
        RedisKey::open_with_flags(self.ctx, key, flags)
    }

    #[must_use]
    pub fn open_key_writable(&self, key: &RedisString) -> RedisKeyWritable {
        RedisKeyWritable::open(self.ctx, key)
    }

    #[must_use]
    pub fn open_key_writable_with_flags(
        &self,
        key: &RedisString,
        flags: KeyFlags,
    ) -> RedisKeyWritable {
        RedisKeyWritable::open_with_flags(self.ctx, key, flags)
    }

    pub fn replicate_verbatim(&self) {
        raw::replicate_verbatim(self.ctx);
    }

    /// Replicate command to the replica and AOF.
    pub fn replicate<'a, T: Into<StrCallArgs<'a>>>(&self, command: &str, args: T) {
        raw::replicate(self.ctx, command, args);
    }

    #[must_use]
    pub fn create_string<T: Into<Vec<u8>>>(&self, s: T) -> RedisString {
        RedisString::create(NonNull::new(self.ctx), s)
    }

    #[must_use]
    pub const fn get_raw(&self) -> *mut raw::RedisModuleCtx {
        self.ctx
    }

    /// # Safety
    ///
    /// See [raw::export_shared_api].
    pub unsafe fn export_shared_api(
        &self,
        func: *const ::std::os::raw::c_void,
        name: *const ::std::os::raw::c_char,
    ) {
        raw::export_shared_api(self.ctx, func, name);
    }

    /// # Safety
    ///
    /// See [raw::notify_keyspace_event].
    #[allow(clippy::must_use_candidate)]
    pub fn notify_keyspace_event(
        &self,
        event_type: raw::NotifyEvent,
        event: &str,
        keyname: &RedisString,
    ) -> raw::Status {
        unsafe { raw::notify_keyspace_event(self.ctx, event_type, event, keyname) }
    }

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

    /// Returns the redis version either by calling `RedisModule_GetServerVersion` API,
    /// Or if it is not available, by calling "info server" API and parsing the reply
    pub fn get_redis_version(&self) -> Result<Version, RedisError> {
        self.get_redis_version_internal(false)
    }

    /// Returns the redis version by calling "info server" API and parsing the reply
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
                    Self::version_from_info(info)
                } else {
                    Err(RedisError::Str("Error calling \"info server\""))
                }
            }
        }
    }
    pub fn set_module_options(&self, options: ModuleOptions) {
        unsafe { raw::RedisModule_SetModuleOptions.unwrap()(self.ctx, options.bits()) };
    }

    /// Return ContextFlags object that allows to check properties related to the state of
    /// the current Redis instance such as:
    /// * Role (master/slave)
    /// * Loading RDB/AOF
    /// * Execution mode such as multi exec or Lua
    pub fn get_flags(&self) -> ContextFlags {
        ContextFlags::from_bits_truncate(unsafe {
            raw::RedisModule_GetContextFlags.unwrap()(self.ctx)
        })
    }

    /// Return the current user name attached to the context
    pub fn get_current_user(&self) -> RedisString {
        let user = unsafe { raw::RedisModule_GetCurrentUserName.unwrap()(self.ctx) };
        RedisString::from_redis_module_string(ptr::null_mut(), user)
    }

    /// Attach the given user to the current context so each operation performed from
    /// now on using this context will be validated againts this new user.
    /// Return [ContextUserScope] which make sure to unset the user when freed and
    /// can not outlive the current [Context].
    pub fn authenticate_user(
        &self,
        user_name: &RedisString,
    ) -> Result<ContextUserScope<'_>, RedisError> {
        let user = unsafe { raw::RedisModule_GetModuleUserFromUserName.unwrap()(user_name.inner) };
        if user.is_null() {
            return Err(RedisError::Str("User does not exists or disabled"));
        }
        unsafe { raw::RedisModule_SetContextUser.unwrap()(self.ctx, user) };
        Ok(ContextUserScope::new(self, user))
    }

    fn deautenticate_user(&self) {
        unsafe { raw::RedisModule_SetContextUser.unwrap()(self.ctx, ptr::null_mut()) };
    }

    /// Verify the the given user has the give ACL permission on the given key.
    /// Return Ok(()) if the user has the permissions or error (with relevant error message)
    /// if the validation failed.
    pub fn acl_check_key_permission(
        &self,
        user_name: &RedisString,
        key_name: &RedisString,
        permissions: &AclPermissions,
    ) -> Result<(), RedisError> {
        let user = unsafe { raw::RedisModule_GetModuleUserFromUserName.unwrap()(user_name.inner) };
        if user.is_null() {
            return Err(RedisError::Str("User does not exists or disabled"));
        }
        let acl_permission_result: raw::Status = unsafe {
            raw::RedisModule_ACLCheckKeyPermissions.unwrap()(
                user,
                key_name.inner,
                permissions.bits(),
            )
        }
        .into();
        unsafe { raw::RedisModule_FreeModuleUser.unwrap()(user) };
        let acl_permission_result: Result<(), &str> = acl_permission_result.into();
        acl_permission_result.map_err(|_e| RedisError::Str("User does not have permissions on key"))
    }

    api!(
        [RedisModule_AddPostNotificationJob],
        /// When running inside a key space notification callback, it is dangerous and highly discouraged to perform any write
        /// operation. In order to still perform write actions in this scenario, Redis provides this API ([add_post_notification_job])
        /// that allows to register a job callback which Redis will call when the following condition holds:
        ///
        /// 1. It is safe to perform any write operation.
        /// 2. The job will be called atomically along side the key space notification.
        ///
        /// Notice, one job might trigger key space notifications that will trigger more jobs.
        /// This raises a concerns of entering an infinite loops, we consider infinite loops
        /// as a logical bug that need to be fixed in the module, an attempt to protect against
        /// infinite loops by halting the execution could result in violation of the feature correctness
        /// and so Redis will make no attempt to protect the module from infinite loops.
        pub fn add_post_notification_job<F: FnOnce(&Context) + 'static>(
            &self,
            callback: F,
        ) -> Status {
            let callback = Box::into_raw(Box::new(Some(callback)));
            unsafe {
                RedisModule_AddPostNotificationJob(
                    self.ctx,
                    Some(post_notification_job::<F>),
                    callback as *mut c_void,
                    Some(post_notification_job_free_callback::<F>),
                )
            }
            .into()
        }
    );

    api!(
        [RedisModule_AvoidReplicaTraffic],
        /// Returns true if a client sent the CLIENT PAUSE command to the server or
        /// if Redis Cluster does a manual failover, pausing the clients.
        /// This is needed when we have a master with replicas, and want to write,
        /// without adding further data to the replication channel, that the replicas
        /// replication offset, match the one of the master. When this happens, it is
        /// safe to failover the master without data loss.
        ///
        /// However modules may generate traffic by calling commands or directly send
        /// data to the replication stream.
        ///
        /// So modules may want to try to avoid very heavy background work that has
        /// the effect of creating data to the replication channel, when this function
        /// returns true. This is mostly useful for modules that have background
        /// garbage collection tasks, or that do writes and replicate such writes
        /// periodically in timer callbacks or other periodic callbacks.
        pub fn avoid_replication_traffic(&self) -> bool {
            unsafe { RedisModule_AvoidReplicaTraffic() == 1 }
        }
    );

    /// Return [Ok(true)] is the current Redis deployment is enterprise, otherwise [Ok(false)].
    /// Return error in case it was not possible to determind the deployment.
    fn is_enterprise_internal(&self) -> Result<bool, RedisError> {
        let info_res = self.call("info", &["server"])?;
        let info = match &info_res {
            RedisValue::BulkRedisString(res) => res.try_as_str()?,
            RedisValue::SimpleString(res) => res.as_str(),
            _ => return Err(RedisError::Str("Mismatch call reply type")),
        };
        Ok(info.contains("rlec_version:"))
    }

    /// Return `true` is the current Redis deployment is enterprise, otherwise `false`.
    pub fn is_enterprise(&self) -> bool {
        self.is_enterprise_internal().unwrap_or_else(|e| {
            log::error!("Failed getting deployment type, assuming oss. Error: {e}.");
            false
        })
    }
}

extern "C" fn post_notification_job_free_callback<F: FnOnce(&Context)>(pd: *mut c_void) {
    drop(unsafe { Box::from_raw(pd as *mut Option<F>) });
}

extern "C" fn post_notification_job<F: FnOnce(&Context)>(
    ctx: *mut raw::RedisModuleCtx,
    pd: *mut c_void,
) {
    let callback = unsafe { &mut *(pd as *mut Option<F>) };
    let ctx = Context::new(ctx);
    callback.take().map_or_else(
        || {
            ctx.log(
                RedisLogLevel::Warning,
                "Got a None callback on post notification job.",
            )
        },
        |callback| {
            callback(&ctx);
        },
    );
}

unsafe impl RedisLockIndicator for Context {}

bitflags! {
    /// An object represent ACL permissions.
    /// Used to check ACL permission using `acl_check_key_permission`.
    #[derive(Debug)]
    pub struct AclPermissions : c_int {
        /// User can look at the content of the value, either return it or copy it.
        const ACCESS = raw::REDISMODULE_CMD_KEY_ACCESS as c_int;

        /// User can insert more data to the key, without deleting or modify existing data.
        const INSERT = raw::REDISMODULE_CMD_KEY_INSERT as c_int;

        /// User can delete content from the key.
        const DELETE = raw::REDISMODULE_CMD_KEY_DELETE as c_int;

        /// User can update existing data inside the key.
        const UPDATE = raw::REDISMODULE_CMD_KEY_UPDATE as c_int;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum AclCategory {
    #[default]
    None,
    Keyspace,
    Read,
    Write,
    Set,
    SortedSet,
    List,
    Hash,
    String,
    Bitmap,
    HyperLogLog,
    Geo,
    Stream,
    PubSub,
    Admin,
    Fast,
    Slow,
    Blocking,
    Dangerous,
    Connection,
    Transaction,
    Scripting,
    Single(String),
    Multi(Vec<AclCategory>),
}

impl From<Vec<AclCategory>> for AclCategory {
    fn from(value: Vec<AclCategory>) -> Self {
        AclCategory::Multi(value)
    }
}

impl From<&str> for AclCategory {
    fn from(value: &str) -> Self {
        match value {
            "" => AclCategory::None,
            "keyspace" => AclCategory::Keyspace,
            "read" => AclCategory::Read,
            "write" => AclCategory::Write,
            "set" => AclCategory::Set,
            "sortedset" => AclCategory::SortedSet,
            "list" => AclCategory::List,
            "hash" => AclCategory::Hash,
            "string" => AclCategory::String,
            "bitmap" => AclCategory::Bitmap,
            "hyperloglog" => AclCategory::HyperLogLog,
            "geo" => AclCategory::Geo,
            "stream" => AclCategory::Stream,
            "pubsub" => AclCategory::PubSub,
            "admin" => AclCategory::Admin,
            "fast" => AclCategory::Fast,
            "slow" => AclCategory::Slow,
            "blocking" => AclCategory::Blocking,
            "dangerous" => AclCategory::Dangerous,
            "connection" => AclCategory::Connection,
            "transaction" => AclCategory::Transaction,
            "scripting" => AclCategory::Scripting,
            _ if !value.contains(" ") => AclCategory::Single(value.to_string()),
            _ => AclCategory::Multi(value.split_whitespace().map(AclCategory::from).collect()),
        }
    }
}

impl From<AclCategory> for String {
    fn from(value: AclCategory) -> Self {
        match value {
            AclCategory::None => "".to_string(),
            AclCategory::Keyspace => "keyspace".to_string(),
            AclCategory::Read => "read".to_string(),
            AclCategory::Write => "write".to_string(),
            AclCategory::Set => "set".to_string(),
            AclCategory::SortedSet => "sortedset".to_string(),
            AclCategory::List => "list".to_string(),
            AclCategory::Hash => "hash".to_string(),
            AclCategory::String => "string".to_string(),
            AclCategory::Bitmap => "bitmap".to_string(),
            AclCategory::HyperLogLog => "hyperloglog".to_string(),
            AclCategory::Geo => "geo".to_string(),
            AclCategory::Stream => "stream".to_string(),
            AclCategory::PubSub => "pubsub".to_string(),
            AclCategory::Admin => "admin".to_string(),
            AclCategory::Fast => "fast".to_string(),
            AclCategory::Slow => "slow".to_string(),
            AclCategory::Blocking => "blocking".to_string(),
            AclCategory::Dangerous => "dangerous".to_string(),
            AclCategory::Connection => "connection".to_string(),
            AclCategory::Transaction => "transaction".to_string(),
            AclCategory::Scripting => "scripting".to_string(),
            AclCategory::Single(s) => s,
            AclCategory::Multi(v) => v
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl std::fmt::Display for AclCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self.clone()))
    }
}

/// The values allowed in the "info" sections and dictionaries.
#[derive(Debug, Clone)]
pub enum InfoContextBuilderFieldBottomLevelValue {
    /// A simple string value.
    String(String),
    /// A numeric value ([`i64`]).
    I64(i64),
    /// A numeric value ([`u64`]).
    U64(u64),
    /// A numeric value ([`f64`]).
    F64(f64),
}

impl From<String> for InfoContextBuilderFieldBottomLevelValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for InfoContextBuilderFieldBottomLevelValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<i64> for InfoContextBuilderFieldBottomLevelValue {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<u64> for InfoContextBuilderFieldBottomLevelValue {
    fn from(value: u64) -> Self {
        Self::U64(value)
    }
}

#[derive(Debug, Clone)]
pub enum InfoContextBuilderFieldTopLevelValue {
    /// A simple bottom-level value.
    Value(InfoContextBuilderFieldBottomLevelValue),
    /// A dictionary value.
    ///
    /// An example of what it looks like:
    /// ```no_run,ignore,
    /// > redis-cli: INFO
    /// >
    /// > # <section name>
    /// <dictionary name>:<key 1>=<value 1>,<key 2>=<value 2>
    /// ```
    ///
    /// Let's suppose we added a section `"my_info"`. Then into this
    /// section we can add a dictionary. Let's add a dictionary named
    /// `"module"`, with with fields `"name"` which is equal to
    /// `"redisgears_2"` and `"ver"` with a value of `999999`. If our
    /// module is named "redisgears_2", we can call `INFO redisgears_2`
    /// to obtain this information:
    ///
    /// ```no_run,ignore,
    /// > redis-cli: INFO redisgears_2
    /// >
    /// > # redisgears_2_my_info
    /// module:name=redisgears_2,ver=999999
    /// ```
    Dictionary {
        name: String,
        fields: InfoContextFieldBottomLevelData,
    },
}

impl<T: Into<InfoContextBuilderFieldBottomLevelValue>> From<T>
    for InfoContextBuilderFieldTopLevelValue
{
    fn from(value: T) -> Self {
        Self::Value(value.into())
    }
}

/// Builds a dictionary within the [`InfoContext`], similar to
/// `INFO KEYSPACE`.
#[derive(Debug)]
pub struct InfoContextBuilderDictionaryBuilder<'a> {
    /// The info section builder this dictionary builder is for.
    info_section_builder: InfoContextBuilderSectionBuilder<'a>,
    /// The name of the section to build.
    name: String,
    /// The fields this section contains.
    fields: InfoContextFieldBottomLevelData,
}

impl<'a> InfoContextBuilderDictionaryBuilder<'a> {
    /// Adds a field within this section.
    pub fn field<F: Into<InfoContextBuilderFieldBottomLevelValue>>(
        mut self,
        name: &str,
        value: F,
    ) -> RedisResult<Self> {
        if self.fields.iter().any(|k| k.0 .0 == name) {
            return Err(RedisError::String(format!(
                "Found duplicate key '{name}' in the info dictionary '{}'",
                self.name
            )));
        }

        self.fields.push((name.to_owned(), value.into()).into());
        Ok(self)
    }

    /// Builds the dictionary with the fields provided.
    pub fn build_dictionary(self) -> RedisResult<InfoContextBuilderSectionBuilder<'a>> {
        let name = self.name;
        let name_ref = name.clone();
        self.info_section_builder.field(
            &name_ref,
            InfoContextBuilderFieldTopLevelValue::Dictionary {
                name,
                fields: self.fields.to_owned(),
            },
        )
    }
}

/// Builds a section within the [`InfoContext`].
#[derive(Debug)]
pub struct InfoContextBuilderSectionBuilder<'a> {
    /// The info builder this section builder is for.
    info_builder: InfoContextBuilder<'a>,
    /// The name of the section to build.
    name: String,
    /// The fields this section contains.
    fields: InfoContextFieldTopLevelData,
}

impl<'a> InfoContextBuilderSectionBuilder<'a> {
    /// Adds a field within this section.
    pub fn field<F: Into<InfoContextBuilderFieldTopLevelValue>>(
        mut self,
        name: &str,
        value: F,
    ) -> RedisResult<Self> {
        if self.fields.iter().any(|(k, _)| k == name) {
            return Err(RedisError::String(format!(
                "Found duplicate key '{name}' in the info section '{}'",
                self.name
            )));
        }
        self.fields.push((name.to_owned(), value.into()));
        Ok(self)
    }

    /// Adds a new dictionary.
    pub fn add_dictionary(self, dictionary_name: &str) -> InfoContextBuilderDictionaryBuilder<'a> {
        InfoContextBuilderDictionaryBuilder {
            info_section_builder: self,
            name: dictionary_name.to_owned(),
            fields: InfoContextFieldBottomLevelData::default(),
        }
    }

    /// Builds the section with the fields provided.
    pub fn build_section(mut self) -> RedisResult<InfoContextBuilder<'a>> {
        if self
            .info_builder
            .sections
            .iter()
            .any(|(k, _)| k == &self.name)
        {
            return Err(RedisError::String(format!(
                "Found duplicate section in the Info reply: {}",
                self.name
            )));
        }

        self.info_builder
            .sections
            .push((self.name.clone(), self.fields));

        Ok(self.info_builder)
    }
}

/// A single info context's bottom level field data.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct InfoContextBottomLevelFieldData(pub (String, InfoContextBuilderFieldBottomLevelValue));
impl Deref for InfoContextBottomLevelFieldData {
    type Target = (String, InfoContextBuilderFieldBottomLevelValue);

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for InfoContextBottomLevelFieldData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Into<InfoContextBuilderFieldBottomLevelValue>> From<(String, T)>
    for InfoContextBottomLevelFieldData
{
    fn from(value: (String, T)) -> Self {
        Self((value.0, value.1.into()))
    }
}
/// A type for the `key => bottom-level-value` storage of an info
/// section.
#[derive(Debug, Default, Clone)]
#[repr(transparent)]
pub struct InfoContextFieldBottomLevelData(pub Vec<InfoContextBottomLevelFieldData>);
impl Deref for InfoContextFieldBottomLevelData {
    type Target = Vec<InfoContextBottomLevelFieldData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for InfoContextFieldBottomLevelData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// A type alias for the `key => top-level-value` storage of an info
/// section.
pub type InfoContextFieldTopLevelData = Vec<(String, InfoContextBuilderFieldTopLevelValue)>;
/// One section contents: name and children.
pub type OneInfoSectionData = (String, InfoContextFieldTopLevelData);
/// A type alias for the section data, associated with the info section.
pub type InfoContextTreeData = Vec<OneInfoSectionData>;

impl<T: Into<InfoContextBuilderFieldBottomLevelValue>> From<BTreeMap<String, T>>
    for InfoContextFieldBottomLevelData
{
    fn from(value: BTreeMap<String, T>) -> Self {
        Self(
            value
                .into_iter()
                .map(|e| (e.0, e.1.into()).into())
                .collect(),
        )
    }
}

impl<T: Into<InfoContextBuilderFieldBottomLevelValue>> From<HashMap<String, T>>
    for InfoContextFieldBottomLevelData
{
    fn from(value: HashMap<String, T>) -> Self {
        Self(
            value
                .into_iter()
                .map(|e| (e.0, e.1.into()).into())
                .collect(),
        )
    }
}

#[derive(Debug)]
pub struct InfoContextBuilder<'a> {
    context: &'a InfoContext,
    sections: InfoContextTreeData,
}
impl<'a> InfoContextBuilder<'a> {
    fn add_bottom_level_field(
        &self,
        key: &str,
        value: &InfoContextBuilderFieldBottomLevelValue,
    ) -> RedisResult<()> {
        use InfoContextBuilderFieldBottomLevelValue as BottomLevel;

        match value {
            BottomLevel::String(string) => add_info_field_str(self.context.ctx, key, string),
            BottomLevel::I64(number) => add_info_field_long_long(self.context.ctx, key, *number),
            BottomLevel::U64(number) => {
                add_info_field_unsigned_long_long(self.context.ctx, key, *number)
            }
            BottomLevel::F64(number) => add_info_field_double(self.context.ctx, key, *number),
        }
        .into()
    }
    /// Adds fields. Make sure that the corresponding section/dictionary
    /// have been added before calling this method.
    fn add_top_level_fields(&self, fields: &InfoContextFieldTopLevelData) -> RedisResult<()> {
        use InfoContextBuilderFieldTopLevelValue as TopLevel;

        fields.iter().try_for_each(|(key, value)| match value {
            TopLevel::Value(bottom_level) => self.add_bottom_level_field(key, bottom_level),
            TopLevel::Dictionary { name, fields } => {
                std::convert::Into::<RedisResult<()>>::into(add_info_begin_dict_field(
                    self.context.ctx,
                    name,
                ))?;
                fields
                    .iter()
                    .try_for_each(|f| self.add_bottom_level_field(&f.0 .0, &f.0 .1))?;
                add_info_end_dict_field(self.context.ctx).into()
            }
        })
    }

    fn finalise_data(&self) -> RedisResult<()> {
        self.sections
            .iter()
            .try_for_each(|(section_name, section_fields)| -> RedisResult<()> {
                if add_info_section(self.context.ctx, Some(section_name)) == Status::Ok {
                    self.add_top_level_fields(section_fields)
                } else {
                    // This section wasn't requested.
                    Ok(())
                }
            })
    }

    /// Sends the info accumulated so far to the [`InfoContext`].
    pub fn build_info(self) -> RedisResult<&'a InfoContext> {
        self.finalise_data().map(|_| self.context)
    }

    /// Returns a section builder.
    pub fn add_section(self, name: &'a str) -> InfoContextBuilderSectionBuilder<'a> {
        InfoContextBuilderSectionBuilder {
            info_builder: self,
            name: name.to_owned(),
            fields: InfoContextFieldTopLevelData::new(),
        }
    }

    /// Adds the section data without checks for the values already
    /// being present. In this case, the values will be overwritten.
    pub(crate) fn add_section_unchecked(mut self, section: OneInfoSectionData) -> Self {
        self.sections.push(section);
        self
    }
}

impl<'a> From<&'a InfoContext> for InfoContextBuilder<'a> {
    fn from(context: &'a InfoContext) -> Self {
        Self {
            context,
            sections: InfoContextTreeData::new(),
        }
    }
}

#[derive(Debug)]
pub struct InfoContext {
    pub ctx: *mut raw::RedisModuleInfoCtx,
}

impl InfoContext {
    pub const fn new(ctx: *mut raw::RedisModuleInfoCtx) -> Self {
        Self { ctx }
    }

    /// Returns a builder for the [`InfoContext`].
    pub fn builder(&self) -> InfoContextBuilder<'_> {
        InfoContextBuilder::from(self)
    }

    /// Returns a build result for the passed [`OneInfoSectionData`].
    pub fn build_one_section<T: Into<OneInfoSectionData>>(&self, data: T) -> RedisResult<()> {
        self.builder()
            .add_section_unchecked(data.into())
            .build_info()?;
        Ok(())
    }

    #[deprecated = "Please use [`InfoContext::builder`] instead."]
    /// The `name` of the sction will be prefixed with the module name
    /// and an underscore: `<module name>_<name>`.
    pub fn add_info_section(&self, name: Option<&str>) -> Status {
        add_info_section(self.ctx, name)
    }

    #[deprecated = "Please use [`InfoContext::builder`] instead."]
    /// The `name` will be prefixed with the module name and an
    /// underscore: `<module name>_<name>`. The `content` pass is left
    /// "as is".
    pub fn add_info_field_str(&self, name: &str, content: &str) -> Status {
        add_info_field_str(self.ctx, name, content)
    }

    #[deprecated = "Please use [`InfoContext::builder`] instead."]
    /// The `name` will be prefixed with the module name and an
    /// underscore: `<module name>_<name>`. The `value` pass is left
    /// "as is".
    pub fn add_info_field_long_long(&self, name: &str, value: c_longlong) -> Status {
        add_info_field_long_long(self.ctx, name, value)
    }
}

bitflags! {
    pub struct ContextFlags : c_int {
        /// The command is running in the context of a Lua script
        const LUA = raw::REDISMODULE_CTX_FLAGS_LUA as c_int;

        /// The command is running inside a Redis transaction
        const MULTI = raw::REDISMODULE_CTX_FLAGS_MULTI as c_int;

        /// The instance is a master
        const MASTER = raw::REDISMODULE_CTX_FLAGS_MASTER as c_int;

        /// The instance is a SLAVE
        const SLAVE = raw::REDISMODULE_CTX_FLAGS_SLAVE as c_int;

        /// The instance is read-only (usually meaning it's a slave as well)
        const READONLY = raw::REDISMODULE_CTX_FLAGS_READONLY as c_int;

        /// The instance is running in cluster mode
        const CLUSTER = raw::REDISMODULE_CTX_FLAGS_CLUSTER as c_int;

        /// The instance has AOF enabled
        const AOF = raw::REDISMODULE_CTX_FLAGS_AOF as c_int;

        /// The instance has RDB enabled
        const RDB = raw::REDISMODULE_CTX_FLAGS_RDB as c_int;

        /// The instance has Maxmemory set
        const MAXMEMORY = raw::REDISMODULE_CTX_FLAGS_MAXMEMORY as c_int;

        /// Maxmemory is set and has an eviction policy that may delete keys
        const EVICTED = raw::REDISMODULE_CTX_FLAGS_EVICT as c_int;

        /// Redis is out of memory according to the maxmemory flag.
        const OOM = raw::REDISMODULE_CTX_FLAGS_OOM as c_int;

        /// Less than 25% of memory available according to maxmemory.
        const OOM_WARNING = raw::REDISMODULE_CTX_FLAGS_OOM_WARNING as c_int;

        /// The command was sent over the replication link.
        const REPLICATED = raw::REDISMODULE_CTX_FLAGS_REPLICATED as c_int;

        /// Redis is currently loading either from AOF or RDB.
        const LOADING = raw::REDISMODULE_CTX_FLAGS_LOADING as c_int;

        /// The replica has no link with its master
        const REPLICA_IS_STALE = raw::REDISMODULE_CTX_FLAGS_REPLICA_IS_STALE as c_int;

        /// The replica is trying to connect with the master
        const REPLICA_IS_CONNECTING = raw::REDISMODULE_CTX_FLAGS_REPLICA_IS_CONNECTING as c_int;

        /// The replica is receiving an RDB file from its master.
        const REPLICA_IS_TRANSFERRING = raw::REDISMODULE_CTX_FLAGS_REPLICA_IS_TRANSFERRING as c_int;

        /// The replica is online, receiving updates from its master
        const REPLICA_IS_ONLINE = raw::REDISMODULE_CTX_FLAGS_REPLICA_IS_ONLINE as c_int;

        /// There is currently some background process active.
        const ACTIVE_CHILD = raw::REDISMODULE_CTX_FLAGS_ACTIVE_CHILD as c_int;

        /// Redis is currently running inside background child process.
        const IS_CHILD = raw::REDISMODULE_CTX_FLAGS_IS_CHILD as c_int;

        /// The next EXEC will fail due to dirty CAS (touched keys).
        const MULTI_DIRTY = raw::REDISMODULE_CTX_FLAGS_MULTI_DIRTY as c_int;

        /// The current client does not allow blocking, either called from
        /// within multi, lua, or from another module using RM_Call
        const DENY_BLOCKING = raw::REDISMODULE_CTX_FLAGS_DENY_BLOCKING as c_int;

        /// The current client uses RESP3 protocol
        const FLAGS_RESP3 = raw::REDISMODULE_CTX_FLAGS_RESP3 as c_int;

        /// Redis is currently async loading database for diskless replication.
        const ASYNC_LOADING = raw::REDISMODULE_CTX_FLAGS_ASYNC_LOADING as c_int;
    }
}
