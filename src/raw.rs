// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

extern crate enum_primitive_derive;
extern crate libc;
extern crate num_traits;

use std::cmp::Ordering;
use std::ffi::{c_ulonglong, CString};
use std::os::raw::{c_char, c_double, c_int, c_long, c_longlong, c_void};
use std::ptr;
use std::slice;

use crate::RedisResult;
use bitflags::bitflags;
use enum_primitive_derive::Primitive;
use libc::size_t;

use crate::error::Error;
pub use crate::redisraw::bindings::*;
use crate::{context::StrCallArgs, RedisString};
use crate::{RedisBuffer, RedisError};

const GENERIC_ERROR_MESSAGE: &str = "Generic error.";

bitflags! {
    pub struct KeyMode: c_int {
        const READ = REDISMODULE_READ as c_int;
        const WRITE = REDISMODULE_WRITE as c_int;
    }
}

bitflags! {
    pub struct ModuleOptions: c_int {
        const HANDLE_IO_ERRORS = REDISMODULE_OPTIONS_HANDLE_IO_ERRORS as c_int;
        const NO_IMPLICIT_SIGNAL_MODIFIED = REDISMODULE_OPTION_NO_IMPLICIT_SIGNAL_MODIFIED as c_int;
        const HANDLE_REPL_ASYNC_LOAD = REDISMODULE_OPTIONS_HANDLE_REPL_ASYNC_LOAD as c_int;
    }
}

/// Gracefully wraps a call to a raw function, trying to convert the
/// result into the suitable return value in the place it is called, if
/// needed.
macro_rules! redis_call {
    // Calls a raw function and simply returns the value as-is.
    (raw $raw_function:ident) => {
        unsafe {
            $raw_function
                .expect(&format!("The function {} is available.", stringify($raw_function)))
                ()
        }
    };

    // Calls a raw function with the arguments provided, attempting to
    // convert the resulting value using the [`TryInto`] trait
    // automatically.
    ($raw_function:ident($($args:expr),*)) => {
        unsafe {
            $raw_function
                .expect(&format!("The function {} is available.", stringify!($raw_function)))
                ($($args),*)
                .try_into()
                .unwrap()
        }
    };

    // Calls a raw function with the arguments provided, attempting to
    // convert the resulting value to the specified type using the
    // [`TryFrom`] trait.
    ($raw_function:ident<$typ:ty>($($args:expr),*)) => {
        unsafe {
            <$typ>::try_from(
                $raw_function
                    .expect(&format!("The function {} is available.", stringify!($raw_function)))
                    ($($args),*)
            )
            .expect(&format!("A conversion to {} being possible.", stringify!($typ)))
        }
    };

    // Calls a raw function with the arguments provided, returning the
    // resulting value as-is.
    (raw $raw_function:ident($($args:expr),*)) => {
        unsafe {
            $raw_function
                .expect(&format!("The function {} is available.", stringify!($raw_function)))
                ($($args),*)
        }
    };
}

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum KeyType {
    Empty = REDISMODULE_KEYTYPE_EMPTY,
    String = REDISMODULE_KEYTYPE_STRING,
    List = REDISMODULE_KEYTYPE_LIST,
    Hash = REDISMODULE_KEYTYPE_HASH,
    Set = REDISMODULE_KEYTYPE_SET,
    ZSet = REDISMODULE_KEYTYPE_ZSET,
    Module = REDISMODULE_KEYTYPE_MODULE,
    Stream = REDISMODULE_KEYTYPE_STREAM,
}

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum Where {
    ListHead = REDISMODULE_LIST_HEAD,
    ListTail = REDISMODULE_LIST_TAIL,
}

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum ReplyType {
    Unknown = REDISMODULE_REPLY_UNKNOWN,
    String = REDISMODULE_REPLY_STRING,
    Error = REDISMODULE_REPLY_ERROR,
    Integer = REDISMODULE_REPLY_INTEGER,
    Array = REDISMODULE_REPLY_ARRAY,
    Null = REDISMODULE_REPLY_NULL,
    Map = REDISMODULE_REPLY_MAP,
    Set = REDISMODULE_REPLY_SET,
    Bool = REDISMODULE_REPLY_BOOL,
    Double = REDISMODULE_REPLY_DOUBLE,
    BigNumber = REDISMODULE_REPLY_BIG_NUMBER,
    VerbatimString = REDISMODULE_REPLY_VERBATIM_STRING,
}

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum Aux {
    Before = REDISMODULE_AUX_BEFORE_RDB,
    After = REDISMODULE_AUX_AFTER_RDB,
}

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = REDISMODULE_OK,
    Err = REDISMODULE_ERR,
}

impl From<Status> for RedisResult<()> {
    fn from(value: Status) -> Self {
        match value {
            Status::Ok => Ok(()),
            Status::Err => Err(RedisError::Str(GENERIC_ERROR_MESSAGE)),
        }
    }
}

impl From<Status> for Result<(), &str> {
    fn from(s: Status) -> Self {
        match s {
            Status::Ok => Ok(()),
            Status::Err => Err(GENERIC_ERROR_MESSAGE),
        }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub struct NotifyEvent : c_int {
        const GENERIC = REDISMODULE_NOTIFY_GENERIC;
        const STRING = REDISMODULE_NOTIFY_STRING;
        const LIST = REDISMODULE_NOTIFY_LIST;
        const SET = REDISMODULE_NOTIFY_SET;
        const HASH = REDISMODULE_NOTIFY_HASH;
        const ZSET = REDISMODULE_NOTIFY_ZSET;
        const EXPIRED = REDISMODULE_NOTIFY_EXPIRED;
        const EVICTED = REDISMODULE_NOTIFY_EVICTED;
        const STREAM = REDISMODULE_NOTIFY_STREAM;
        const MODULE = REDISMODULE_NOTIFY_MODULE;
        const LOADED = REDISMODULE_NOTIFY_LOADED;
        const MISSED = REDISMODULE_NOTIFY_KEY_MISS;
        const ALL = REDISMODULE_NOTIFY_ALL;
        const TRIMMED = REDISMODULE_NOTIFY_TRIMMED;
    }
}

#[derive(Debug)]
pub enum CommandFlag {
    Write,
    Readonly,
    Denyoom,
    Admin,
    Pubsub,
    Noscript,
    Random,
    SortForScript,
    Loading,
    Stale,
    SkipMonitor,
    Asking,
    Fast,
    Movablekeys,
}

const fn command_flag_repr(flag: &CommandFlag) -> &'static str {
    use crate::raw::CommandFlag::*;
    match flag {
        Write => "write",
        Readonly => "readonly",
        Denyoom => "denyoom",
        Admin => "admin",
        Pubsub => "pubsub",
        Noscript => "noscript",
        Random => "random",
        SortForScript => "sort_for_script",
        Loading => "loading",
        Stale => "stale",
        SkipMonitor => "skip_monitor",
        Asking => "asking",
        Fast => "fast",
        Movablekeys => "movablekeys",
    }
}

// This is the one static function we need to initialize a module.
// bindgen does not generate it for us (probably since it's defined as static in redismodule.h).
#[allow(improper_ctypes)]
#[link(name = "redismodule", kind = "static")]
extern "C" {
    pub fn Export_RedisModule_Init(
        ctx: *mut RedisModuleCtx,
        module_name: *const c_char,
        module_version: c_int,
        api_version: c_int,
    ) -> c_int;

    pub fn Export_RedisModule_InitAPI(ctx: *mut RedisModuleCtx) -> c_void;
}

///////////////////////////////////////////////////////////////

pub const FMT: *const c_char = b"v\0".as_ptr().cast::<c_char>();

// REDISMODULE_HASH_DELETE is defined explicitly here because bindgen cannot
// parse typecasts in C macro constants yet.
// See https://github.com/rust-lang/rust-bindgen/issues/316
pub const REDISMODULE_HASH_DELETE: *const RedisModuleString = 1 as *const RedisModuleString;

// Helper functions for the raw bindings.

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_type(reply: *mut RedisModuleCallReply) -> ReplyType {
    // TODO: Cache the unwrapped functions and use them instead of unwrapping every time?
    redis_call!(RedisModule_CallReplyType(reply))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn free_call_reply(reply: *mut RedisModuleCallReply) {
    redis_call!(RedisModule_FreeCallReply(reply))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_integer(reply: *mut RedisModuleCallReply) -> c_longlong {
    redis_call!(RedisModule_CallReplyInteger(reply))
}

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_bool(reply: *mut RedisModuleCallReply) -> bool {
    redis_call!(raw RedisModule_CallReplyBool(reply)) != 0
}

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_double(reply: *mut RedisModuleCallReply) -> f64 {
    redis_call!(RedisModule_CallReplyDouble(reply))
}

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_big_number(reply: *mut RedisModuleCallReply) -> Option<String> {
    let mut len: size_t = 0;
    String::from_utf8({
        let reply_string =
            redis_call!(raw RedisModule_CallReplyBigNumber(reply, &mut len)) as *mut u8;
        if reply_string.is_null() {
            return None;
        }
        unsafe { slice::from_raw_parts(reply_string, len).to_vec() }
    })
    .ok()
}

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_verbatim_string(reply: *mut RedisModuleCallReply) -> Option<(String, Vec<u8>)> {
    let mut len: size_t = 0;
    let format: *const u8 = ptr::null();
    let reply_string = redis_call!(raw RedisModule_CallReplyVerbatim(reply, &mut len, &mut (format as *const c_char)))
        as *mut u8;
    if reply_string.is_null() {
        return None;
    }
    Some(unsafe {
        (
            String::from_utf8(slice::from_raw_parts(format, 3).to_vec()).ok()?,
            slice::from_raw_parts(reply_string, len).to_vec(),
        )
    })
}

/// Aborts the invocation of the blocking commands.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_promise_abort(
    reply: *mut RedisModuleCallReply,
    private_data: *mut *mut ::std::os::raw::c_void,
) -> Status {
    redis_call!(RedisModule_CallReplyPromiseAbort(reply, private_data))
}

macro_rules! generate_transparent_binding {
    ($(#[$outer:meta])* $vis:vis $rust_name:ident => $raw_function:ident, $ret_type:ty, $(($arg:ident, $typ:ty)),*) => {
        $(#[$outer])*
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        $vis fn $rust_name($($arg: $typ),*) -> $ret_type {
            raw_call!($raw_function, $($arg),*)
        }
    };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_array_element(
    reply: *mut RedisModuleCallReply,
    idx: usize,
) -> *mut RedisModuleCallReply {
    redis_call!(RedisModule_CallReplyArrayElement(reply, idx))
}

// generate_transparent_binding!(
//     /// # Panics
//     ///
//     /// Panics if the Redis server doesn't support replying with bool (since RESP3).
//     pub call_reply_set_element => RedisModule_CallReplySetElement,
//     *mut RedisModuleCallReply,
//     (reply, *mut RedisModuleCallReply),
//     (idx, usize)
// );

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_set_element(
    reply: *mut RedisModuleCallReply,
    idx: usize,
) -> *mut RedisModuleCallReply {
    redis_call!(RedisModule_CallReplySetElement(reply, idx))
}

/// # Panics
///
/// Panics if the Redis server doesn't support replying with bool (since RESP3).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_map_element(
    reply: *mut RedisModuleCallReply,
    idx: usize,
) -> (*mut RedisModuleCallReply, *mut RedisModuleCallReply) {
    let mut key: *mut RedisModuleCallReply = ptr::null_mut();
    let mut val: *mut RedisModuleCallReply = ptr::null_mut();
    redis_call!(raw RedisModule_CallReplyMapElement(
        reply, idx, &mut key, &mut val
    ));
    (key, val)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_length(reply: *mut RedisModuleCallReply) -> usize {
    redis_call!(raw RedisModule_CallReplyLength(reply))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_string_ptr(reply: *mut RedisModuleCallReply, len: *mut size_t) -> *const c_char {
    redis_call!(raw RedisModule_CallReplyStringPtr(reply, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_string(reply: *mut RedisModuleCallReply) -> Option<String> {
    let mut len: size_t = 0;
    let reply_string: *mut u8 =
        redis_call!(raw RedisModule_CallReplyStringPtr(reply, &mut len)) as *mut u8;
    if reply_string.is_null() {
        return None;
    }
    unsafe { String::from_utf8(slice::from_raw_parts(reply_string, len).to_vec()).ok() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn close_key(kp: *mut RedisModuleKey) {
    redis_call!(RedisModule_CloseKey(kp))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn open_key(
    ctx: *mut RedisModuleCtx,
    keyname: *mut RedisModuleString,
    mode: KeyMode,
) -> *mut RedisModuleKey {
    redis_call!(RedisModule_OpenKey(ctx, keyname, mode.bits()))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub(crate) fn open_key_with_flags(
    ctx: *mut RedisModuleCtx,
    keyname: *mut RedisModuleString,
    mode: KeyMode,
    flags: c_int,
) -> *mut RedisModuleKey {
    redis_call!(RedisModule_OpenKey(ctx, keyname, mode.bits() | flags))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_array(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    redis_call!(RedisModule_ReplyWithArray(ctx, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_map(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    unsafe { RedisModule_ReplyWithMap }
        .map_or_else(
            || redis_call!(RedisModule_ReplyWithArray(ctx, len * 2)),
            |f| unsafe { f(ctx, len) },
        )
        .try_into()
        .unwrap()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_set(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    unsafe { RedisModule_ReplyWithSet }
        .map_or_else(
            || redis_call!(RedisModule_ReplyWithArray(ctx, len * 2)),
            |f| unsafe { f(ctx, len) },
        )
        .try_into()
        .unwrap()
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_attribute(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    redis_call!(RedisModule_ReplyWithAttribute(ctx, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_error<S: AsRef<str>>(ctx: *mut RedisModuleCtx, err: S) -> Status {
    let msg = CString::new(err.as_ref()).unwrap();
    redis_call!(RedisModule_ReplyWithError(ctx, msg.as_ptr()))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_null(ctx: *mut RedisModuleCtx) -> Status {
    redis_call!(RedisModule_ReplyWithNull(ctx))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_bool(ctx: *mut RedisModuleCtx, b: c_int) -> Status {
    redis_call!(RedisModule_ReplyWithBool(ctx, b))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_long_long(ctx: *mut RedisModuleCtx, ll: c_longlong) -> Status {
    redis_call!(RedisModule_ReplyWithLongLong(ctx, ll))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_double(ctx: *mut RedisModuleCtx, f: c_double) -> Status {
    redis_call!(RedisModule_ReplyWithDouble(ctx, f))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_string(ctx: *mut RedisModuleCtx, s: *mut RedisModuleString) -> Status {
    redis_call!(RedisModule_ReplyWithString(ctx, s))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_simple_string(ctx: *mut RedisModuleCtx, s: *const c_char) -> Status {
    redis_call!(RedisModule_ReplyWithSimpleString(ctx, s))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_string_buffer(ctx: *mut RedisModuleCtx, s: *const c_char, len: size_t) -> Status {
    redis_call!(RedisModule_ReplyWithStringBuffer(ctx, s, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_big_number(ctx: *mut RedisModuleCtx, s: *const c_char, len: size_t) -> Status {
    redis_call!(RedisModule_ReplyWithBigNumber(ctx, s, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn reply_with_verbatim_string(
    ctx: *mut RedisModuleCtx,
    s: *const c_char,
    len: size_t,
    format: *const c_char,
) -> Status {
    redis_call!(RedisModule_ReplyWithVerbatimStringType(ctx, s, len, format))
}

// Sets the expiry on a key.
//
// Expire is in milliseconds.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn set_expire(key: *mut RedisModuleKey, expire: c_longlong) -> Status {
    redis_call!(RedisModule_SetExpire(key, expire))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_dma(key: *mut RedisModuleKey, len: *mut size_t, mode: KeyMode) -> *mut c_char {
    redis_call!(RedisModule_StringDMA(key, len, mode.bits()))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_truncate(key: *mut RedisModuleKey, new_len: size_t) -> Status {
    redis_call!(RedisModule_StringTruncate(key, new_len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn hash_get_multi<T>(
    key: *mut RedisModuleKey,
    fields: &[T],
    values: &mut [*mut RedisModuleString],
) -> Result<(), RedisError>
where
    T: Into<Vec<u8>> + Clone,
{
    assert_eq!(fields.len(), values.len());

    let fields = fields
        .iter()
        .map(|e| CString::new(e.clone()))
        .collect::<Result<Vec<CString>, _>>()?;

    let mut fi = fields.iter();
    let mut vi = values.iter_mut();

    macro_rules! rm {
        () => {
            redis_call!(RedisModule_HashGet(key, REDISMODULE_HASH_CFIELDS as i32,
                                         ptr::null::<c_char>()))
        };
        ($($args:expr)*) => {
            redis_call!(RedisModule_HashGet(
                key, REDISMODULE_HASH_CFIELDS as i32,
                $($args),*,
                ptr::null::<c_char>()
            ))
        };
    }
    macro_rules! f {
        () => {
            fi.next().unwrap().as_ptr()
        };
    }
    macro_rules! v {
        () => {
            vi.next().unwrap()
        };
    }

    // This convoluted code is necessary since Redis only exposes a varargs API for HashGet
    // to modules. Unfortunately there's no straightforward or portable way of calling a
    // a varargs function with a variable number of arguments that is determined at runtime.
    // See also the following Redis ticket: https://github.com/redis/redis/issues/7860
    let res = match fields.len() {
        0 => rm! {},
        1 => rm! {f!() v!()},
        2 => rm! {f!() v!() f!() v!()},
        3 => rm! {f!() v!() f!() v!() f!() v!()},
        4 => rm! {f!() v!() f!() v!() f!() v!() f!() v!()},
        5 => rm! {f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()},
        6 => rm! {f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()},
        7 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!()
        },
        8 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!() f!() v!()
        },
        9 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!() f!() v!() f!() v!()
        },
        10 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!() f!() v!() f!() v!() f!() v!()
        },
        11 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
        },
        12 => rm! {
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
            f!() v!() f!() v!() f!() v!() f!() v!() f!() v!() f!() v!()
        },
        _ => panic!("Unsupported length"),
    };

    match res {
        Status::Ok => Ok(()),
        Status::Err => Err(RedisError::Str("ERR key is not a hash value")),
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn hash_set(key: *mut RedisModuleKey, field: &str, value: *mut RedisModuleString) -> Status {
    let field = CString::new(field).unwrap();

    redis_call!(RedisModule_HashSet(
        key,
        REDISMODULE_HASH_CFIELDS as i32,
        field.as_ptr(),
        value,
        ptr::null::<c_char>()
    ))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn hash_del(key: *mut RedisModuleKey, field: &str) -> Status {
    let field = CString::new(field).unwrap();

    // TODO: Add hash_del_multi()
    // Support to pass multiple fields is desired but is complicated.
    // See hash_get_multi() and https://github.com/redis/redis/issues/7860

    redis_call!(RedisModule_HashSet(
        key,
        REDISMODULE_HASH_CFIELDS as i32,
        field.as_ptr(),
        REDISMODULE_HASH_DELETE,
        ptr::null::<c_char>()
    ))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn list_push(
    key: *mut RedisModuleKey,
    list_where: Where,
    element: *mut RedisModuleString,
) -> Status {
    redis_call!(RedisModule_ListPush(key, list_where as i32, element))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn list_pop(key: *mut RedisModuleKey, list_where: Where) -> *mut RedisModuleString {
    redis_call!(RedisModule_ListPop(key, list_where as i32))
}

// Returns pointer to the C string, and sets len to its length
#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_ptr_len(s: *const RedisModuleString, len: *mut size_t) -> *const c_char {
    redis_call!(RedisModule_StringPtrLen(s, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_retain_string(ctx: *mut RedisModuleCtx, s: *mut RedisModuleString) {
    redis_call!(RedisModule_RetainString(ctx, s))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_to_longlong(s: *const RedisModuleString, len: *mut i64) -> Status {
    redis_call!(RedisModule_StringToLongLong(s, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_to_double(s: *const RedisModuleString, len: *mut f64) -> Status {
    redis_call!(RedisModule_StringToDouble(s, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn string_set(key: *mut RedisModuleKey, s: *mut RedisModuleString) -> Status {
    redis_call!(RedisModule_StringSet(key, s))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[inline]
pub fn replicate_verbatim(ctx: *mut RedisModuleCtx) -> Status {
    redis_call!(RedisModule_ReplicateVerbatim(ctx))
}

fn load<F, T>(rdb: *mut RedisModuleIO, f: F) -> Result<T, Error>
where
    F: FnOnce(*mut RedisModuleIO) -> T,
{
    let res = f(rdb);
    if is_io_error(rdb) {
        Err(RedisError::short_read().into())
    } else {
        Ok(res)
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_unsigned(rdb: *mut RedisModuleIO) -> Result<u64, Error> {
    load(rdb, |rdb| redis_call!(RedisModule_LoadUnsigned(rdb)))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_signed(rdb: *mut RedisModuleIO) -> Result<i64, Error> {
    load(rdb, |rdb| redis_call!(RedisModule_LoadSigned(rdb)))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_string(rdb: *mut RedisModuleIO) -> Result<RedisString, Error> {
    let p = load(rdb, |rdb| redis_call!(RedisModule_LoadString(rdb)))?;
    Ok(RedisString::from_redis_module_string(ptr::null_mut(), p))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_string_buffer(rdb: *mut RedisModuleIO) -> Result<RedisBuffer, Error> {
    let mut len = 0;
    let buffer = load(rdb, |rdb| {
        redis_call!(RedisModule_LoadStringBuffer(rdb, &mut len))
    })?;
    Ok(RedisBuffer::new(buffer, len))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn replicate<'a, T: Into<StrCallArgs<'a>>>(
    ctx: *mut RedisModuleCtx,
    command: &str,
    args: T,
) -> Status {
    let mut call_args: StrCallArgs = args.into();
    let final_args = call_args.args_mut();

    let cmd = CString::new(command).unwrap();

    redis_call!(RedisModule_Replicate(
        ctx,
        cmd.as_ptr(),
        FMT,
        final_args.as_ptr(),
        final_args.len()
    ))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_double(rdb: *mut RedisModuleIO) -> Result<f64, Error> {
    load(rdb, |rdb| redis_call!(RedisModule_LoadDouble(rdb)))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_float(rdb: *mut RedisModuleIO) -> Result<f32, Error> {
    load(rdb, |rdb| redis_call!(RedisModule_LoadFloat(rdb)))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_string(rdb: *mut RedisModuleIO, buf: &str) {
    redis_call!(raw RedisModule_SaveStringBuffer(
        rdb,
        buf.as_ptr().cast::<c_char>(),
        buf.len()
    ));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Save the `RedisString` into the RDB
pub fn save_redis_string(rdb: *mut RedisModuleIO, s: &RedisString) {
    redis_call!(raw RedisModule_SaveString(rdb, s.inner));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
/// Save the `&[u8]` into the RDB
pub fn save_slice(rdb: *mut RedisModuleIO, buf: &[u8]) {
    redis_call!(raw RedisModule_SaveStringBuffer(rdb, buf.as_ptr().cast::<c_char>(), buf.len()));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_double(rdb: *mut RedisModuleIO, val: f64) {
    redis_call!(raw RedisModule_SaveDouble(rdb, val));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_signed(rdb: *mut RedisModuleIO, val: i64) {
    redis_call!(raw RedisModule_SaveSigned(rdb, val));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_float(rdb: *mut RedisModuleIO, val: f32) {
    redis_call!(raw RedisModule_SaveFloat(rdb, val));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_unsigned(rdb: *mut RedisModuleIO, val: u64) {
    redis_call!(raw RedisModule_SaveUnsigned(rdb, val));
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_compare(a: *mut RedisModuleString, b: *mut RedisModuleString) -> Ordering {
    redis_call!(raw RedisModule_StringCompare(a, b)).cmp(&0)
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_append_buffer(
    ctx: *mut RedisModuleCtx,
    s: *mut RedisModuleString,
    buff: &str,
) -> Status {
    redis_call!(RedisModule_StringAppendBuffer(
        ctx,
        s,
        buff.as_ptr().cast::<c_char>(),
        buff.len()
    ))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn subscribe_to_server_event(
    ctx: *mut RedisModuleCtx,
    event: RedisModuleEvent,
    callback: RedisModuleEventCallback,
) -> Status {
    redis_call!(RedisModule_SubscribeToServerEvent(ctx, event, callback))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn register_info_function(ctx: *mut RedisModuleCtx, callback: RedisModuleInfoFunc) -> Status {
    redis_call!(RedisModule_RegisterInfoFunc(ctx, callback))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_section(ctx: *mut RedisModuleInfoCtx, name: Option<&str>) -> Status {
    name.map(|n| CString::new(n).unwrap()).map_or_else(
        || redis_call!(RedisModule_InfoAddSection(ctx, ptr::null_mut())),
        |n| redis_call!(RedisModule_InfoAddSection(ctx, n.as_ptr())),
    )
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_str(ctx: *mut RedisModuleInfoCtx, name: &str, content: &str) -> Status {
    let name = CString::new(name).unwrap();
    let content = RedisString::create(None, content);
    redis_call!(RedisModule_InfoAddFieldString(
        ctx,
        name.as_ptr(),
        content.inner
    ))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_long_long(
    ctx: *mut RedisModuleInfoCtx,
    name: &str,
    value: c_longlong,
) -> Status {
    let name = CString::new(name).unwrap();
    redis_call!(RedisModule_InfoAddFieldLongLong(ctx, name.as_ptr(), value))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_unsigned_long_long(
    ctx: *mut RedisModuleInfoCtx,
    name: &str,
    value: c_ulonglong,
) -> Status {
    let name = CString::new(name).unwrap();
    redis_call!(RedisModule_InfoAddFieldULongLong(ctx, name.as_ptr(), value))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_double(ctx: *mut RedisModuleInfoCtx, name: &str, value: c_double) -> Status {
    let name = CString::new(name).unwrap();
    redis_call!(RedisModule_InfoAddFieldDouble(ctx, name.as_ptr(), value))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_begin_dict_field(ctx: *mut RedisModuleInfoCtx, name: &str) -> Status {
    let name = CString::new(name).unwrap();
    redis_call!(RedisModule_InfoBeginDictField(ctx, name.as_ptr()))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_end_dict_field(ctx: *mut RedisModuleInfoCtx) -> Status {
    redis_call!(RedisModule_InfoEndDictField(ctx))
}

/// # Panics
///
/// Panics when the [RedisModule_ExportSharedAPI] is unavailable.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn export_shared_api(
    ctx: *mut RedisModuleCtx,
    func: *const ::std::os::raw::c_void,
    name: *const ::std::os::raw::c_char,
) {
    redis_call!(raw RedisModule_ExportSharedAPI(ctx, name, func as *mut _));
}

/// # Safety
///
/// This function is safe to use as it doesn't perform any work with
/// the [RedisModuleCtx] pointer except for passing it to the redis server.
///
/// # Panics
///
/// Panics when the [RedisModule_NotifyKeyspaceEvent] is unavailable.
pub unsafe fn notify_keyspace_event(
    ctx: *mut RedisModuleCtx,
    event_type: NotifyEvent,
    event: &str,
    keyname: &RedisString,
) -> Status {
    let event = CString::new(event).unwrap();
    redis_call!(RedisModule_NotifyKeyspaceEvent(
        ctx,
        event_type.bits(),
        event.as_ptr(),
        keyname.inner
    ))
}

/// # Panics
///
/// Panics when the [RedisModule_GetNotifyKeyspaceEvents] is unavailable.
pub fn get_keyspace_events() -> NotifyEvent {
    let events = redis_call!(RedisModule_GetNotifyKeyspaceEvents());
    NotifyEvent::from_bits_truncate(events)
}

/// Starts the stream iterator.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn stream_iterator_start(
    key: *mut RedisModuleKey,
    flags: ::std::os::raw::c_int,
    start_id: *mut RedisModuleStreamID,
    end_id: *mut RedisModuleStreamID,
) -> Status {
    redis_call!(RedisModule_StreamIteratorStart(
        key, flags, start_id, end_id
    ))
}

/// Modifies the id passed to point to the next item within the stream.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn stream_iterator_next_id(
    key: *mut RedisModuleKey,
    id: *mut RedisModuleStreamID,
    fields_count: &mut ::std::os::raw::c_long,
) -> Status {
    redis_call!(RedisModule_StreamIteratorNextID(key, id, fields_count))
}

/// Obtains a pointer to the key -> value pair within the stream using
/// the iterator.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn stream_iterator_next_field(
    key: *mut RedisModuleKey,
    field_ptr: *mut *mut RedisModuleString,
    value_ptr: *mut *mut RedisModuleString,
) -> Status {
    redis_call!(RedisModule_StreamIteratorNextField(
        key, field_ptr, value_ptr
    ))
}

/// Deletes the stream iterator accessible by key.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn stream_iterator_delete(key: *mut RedisModuleKey) -> Status {
    redis_call!(RedisModule_StreamIteratorDelete(key))
}

/// Returns the timer information.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_timer_info(
    ctx: *mut RedisModuleCtx,
    id: RedisModuleTimerID,
    remaining: *mut u64,
    data: *mut *mut ::std::os::raw::c_void,
) -> Status {
    redis_call!(RedisModule_GetTimerInfo(ctx, id, remaining, data))
}

/// Stops the timer with the provided id.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn stop_timer(
    ctx: *mut RedisModuleCtx,
    id: RedisModuleTimerID,
    data: *mut *mut ::std::os::raw::c_void,
) -> Status {
    redis_call!(RedisModule_StopTimer(ctx, id, data))
}

/// Reports the "wrong arity" redis error.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn wrong_arity(ctx: *mut RedisModuleCtx) -> Status {
    redis_call!(RedisModule_WrongArity(ctx))
}

/// Checks the passed ACL permissions.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn acl_check_key_permissions(
    user: *mut RedisModuleUser,
    key: *mut RedisModuleString,
    flags: ::std::os::raw::c_int,
) -> Status {
    redis_call!(RedisModule_ACLCheckKeyPermissions(user, key, flags))
}

/// Adds a post notification job
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_post_notification_job(
    ctx: *mut RedisModuleCtx,
    callback: RedisModulePostNotificationJobFunc,
    pd: *mut ::std::os::raw::c_void,
    free_pd: ::std::option::Option<unsafe extern "C" fn(arg1: *mut ::std::os::raw::c_void)>,
) -> Status {
    redis_call!(RedisModule_AddPostNotificationJob(
        ctx, callback, pd, free_pd
    ))
}

/// Returns the module user object for the user name passed.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn get_module_user_from_user_name(name: *mut RedisModuleString) -> *mut RedisModuleUser {
    redis_call!(RedisModule_GetModuleUserFromUserName(name))
}

/// Returns the key type.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn key_type(key: *mut RedisModuleKey) -> KeyType {
    redis_call!(RedisModule_KeyType(key))
}

/// If the key is open for writing, set the specified module type object
/// as the value of the key, deleting the old value if any.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn module_type_set_value(
    key: *mut RedisModuleKey,
    mt: *mut RedisModuleType,
    value: *mut ::std::os::raw::c_void,
) -> Status {
    redis_call!(RedisModule_ModuleTypeSetValue(key, mt, value))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub patch: i32,
}

impl From<c_int> for Version {
    fn from(ver: c_int) -> Self {
        // Expected format: 0x00MMmmpp for Major, minor, patch
        Self {
            major: (ver & 0x00FF_0000) >> 16,
            minor: (ver & 0x0000_FF00) >> 8,
            patch: (ver & 0x0000_00FF),
        }
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn is_io_error(rdb: *mut RedisModuleIO) -> bool {
    redis_call!(raw RedisModule_IsIOError(rdb)) != 0
}
