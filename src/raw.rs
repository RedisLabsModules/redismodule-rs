// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

extern crate enum_primitive_derive;
extern crate libc;
extern crate num_traits;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_double, c_int, c_long, c_longlong};
use std::ptr;
use std::slice;

use bitflags::bitflags;
use enum_primitive_derive::Primitive;
use libc::size_t;
use num_traits::FromPrimitive;

use crate::error::Error;
pub use crate::redisraw::bindings::*;
use crate::{Context, RedisString};
use crate::{RedisBuffer, RedisError};

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
    }
}

#[derive(Primitive, Debug, PartialEq)]
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

impl From<c_int> for KeyType {
    fn from(v: c_int) -> Self {
        Self::from_i32(v).unwrap()
    }
}

#[derive(Primitive, Debug, PartialEq)]
pub enum Where {
    ListHead = REDISMODULE_LIST_HEAD,
    ListTail = REDISMODULE_LIST_TAIL,
}

#[derive(Primitive, Debug, PartialEq)]
pub enum ReplyType {
    Unknown = REDISMODULE_REPLY_UNKNOWN,
    String = REDISMODULE_REPLY_STRING,
    Error = REDISMODULE_REPLY_ERROR,
    Integer = REDISMODULE_REPLY_INTEGER,
    Array = REDISMODULE_REPLY_ARRAY,
    Null = REDISMODULE_REPLY_NULL,
}

impl From<c_int> for ReplyType {
    fn from(v: c_int) -> Self {
        Self::from_i32(v).unwrap()
    }
}

#[derive(Primitive, Debug, PartialEq)]
pub enum Aux {
    Before = REDISMODULE_AUX_BEFORE_RDB,
    After = REDISMODULE_AUX_AFTER_RDB,
}

#[derive(Primitive, Debug, PartialEq)]
pub enum Status {
    Ok = REDISMODULE_OK,
    Err = REDISMODULE_ERR,
}

impl From<c_int> for Status {
    fn from(v: c_int) -> Self {
        Self::from_i32(v).unwrap()
    }
}

impl From<Status> for Result<(), &str> {
    fn from(s: Status) -> Self {
        match s {
            Status::Ok => Ok(()),
            Status::Err => Err("Generic error"),
        }
    }
}

#[cfg(feature = "experimental-api")]
bitflags! {
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
        const ALL = REDISMODULE_NOTIFY_ALL;
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

fn command_flag_repr(flag: &CommandFlag) -> &'static str {
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
    unsafe {
        // TODO: Cache the unwrapped functions and use them instead of unwrapping every time?
        RedisModule_CallReplyType.unwrap()(reply).into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn free_call_reply(reply: *mut RedisModuleCallReply) {
    unsafe { RedisModule_FreeCallReply.unwrap()(reply) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_integer(reply: *mut RedisModuleCallReply) -> c_longlong {
    unsafe { RedisModule_CallReplyInteger.unwrap()(reply) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_array_element(
    reply: *mut RedisModuleCallReply,
    idx: usize,
) -> *mut RedisModuleCallReply {
    unsafe { RedisModule_CallReplyArrayElement.unwrap()(reply, idx) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_length(reply: *mut RedisModuleCallReply) -> usize {
    unsafe { RedisModule_CallReplyLength.unwrap()(reply) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_string_ptr(reply: *mut RedisModuleCallReply, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_CallReplyStringPtr.unwrap()(reply, len) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn call_reply_string(reply: *mut RedisModuleCallReply) -> String {
    unsafe {
        let mut len: size_t = 0;
        let reply_string: *mut u8 =
            RedisModule_CallReplyStringPtr.unwrap()(reply, &mut len) as *mut u8;
        String::from_utf8(
            slice::from_raw_parts(reply_string, len)
                .iter()
                .copied()
                .collect(),
        )
        .unwrap()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn close_key(kp: *mut RedisModuleKey) {
    unsafe { RedisModule_CloseKey.unwrap()(kp) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn open_key(
    ctx: *mut RedisModuleCtx,
    keyname: *mut RedisModuleString,
    mode: KeyMode,
) -> *mut RedisModuleKey {
    unsafe { RedisModule_OpenKey.unwrap()(ctx, keyname, mode.bits).cast::<RedisModuleKey>() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_array(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    unsafe { RedisModule_ReplyWithArray.unwrap()(ctx, len).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_error(ctx: *mut RedisModuleCtx, err: *const c_char) {
    unsafe {
        let msg = Context::str_as_legal_resp_string(CStr::from_ptr(err).to_str().unwrap());
        RedisModule_ReplyWithError.unwrap()(ctx, msg.as_ptr());
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_long_long(ctx: *mut RedisModuleCtx, ll: c_longlong) -> Status {
    unsafe { RedisModule_ReplyWithLongLong.unwrap()(ctx, ll).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_double(ctx: *mut RedisModuleCtx, f: c_double) -> Status {
    unsafe { RedisModule_ReplyWithDouble.unwrap()(ctx, f).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn reply_with_string(ctx: *mut RedisModuleCtx, s: *mut RedisModuleString) -> Status {
    unsafe { RedisModule_ReplyWithString.unwrap()(ctx, s).into() }
}

// Sets the expiry on a key.
//
// Expire is in milliseconds.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn set_expire(key: *mut RedisModuleKey, expire: c_longlong) -> Status {
    unsafe { RedisModule_SetExpire.unwrap()(key, expire).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_dma(key: *mut RedisModuleKey, len: *mut size_t, mode: KeyMode) -> *const c_char {
    unsafe { RedisModule_StringDMA.unwrap()(key, len, mode.bits) }
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

    let mut fi = fields.iter();
    let mut vi = values.iter_mut();

    macro_rules! rm {
        () => { unsafe {
            RedisModule_HashGet.unwrap()(key, REDISMODULE_HASH_CFIELDS as i32,
                                         ptr::null::<c_char>())
        }};
        ($($args:expr)*) => { unsafe {
            RedisModule_HashGet.unwrap()(
                key, REDISMODULE_HASH_CFIELDS as i32,
                $($args),*,
                ptr::null::<c_char>()
            )
        }};
    }
    macro_rules! f {
        () => {
            CString::new((*fi.next().unwrap()).clone())
                .unwrap()
                .as_ptr()
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
    let res = Status::from(match fields.len() {
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
    });

    match res {
        Status::Ok => Ok(()),
        Status::Err => Err(RedisError::Str("ERR key is not a hash value")),
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn hash_set(key: *mut RedisModuleKey, field: &str, value: *mut RedisModuleString) -> Status {
    let field = CString::new(field).unwrap();

    unsafe {
        RedisModule_HashSet.unwrap()(
            key,
            REDISMODULE_HASH_CFIELDS as i32,
            field.as_ptr(),
            value,
            ptr::null::<c_char>(),
        )
        .into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn hash_del(key: *mut RedisModuleKey, field: &str) -> Status {
    let field = CString::new(field).unwrap();

    // TODO: Add hash_del_multi()
    // Support to pass multiple fields is desired but is complicated.
    // See hash_get_multi() and https://github.com/redis/redis/issues/7860

    unsafe {
        RedisModule_HashSet.unwrap()(
            key,
            REDISMODULE_HASH_CFIELDS as i32,
            field.as_ptr(),
            REDISMODULE_HASH_DELETE,
            ptr::null::<c_char>(),
        )
        .into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn list_push(
    key: *mut RedisModuleKey,
    list_where: Where,
    element: *mut RedisModuleString,
) -> Status {
    unsafe { RedisModule_ListPush.unwrap()(key, list_where as i32, element).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn list_pop(key: *mut RedisModuleKey, list_where: Where) -> *mut RedisModuleString {
    unsafe { RedisModule_ListPop.unwrap()(key, list_where as i32) }
}

// Returns pointer to the C string, and sets len to its length
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_ptr_len(s: *const RedisModuleString, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_StringPtrLen.unwrap()(s, len) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_retain_string(ctx: *mut RedisModuleCtx, s: *mut RedisModuleString) {
    unsafe { RedisModule_RetainString.unwrap()(ctx, s) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_to_longlong(s: *const RedisModuleString, len: *mut i64) -> Status {
    unsafe { RedisModule_StringToLongLong.unwrap()(s, len).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_to_double(s: *const RedisModuleString, len: *mut f64) -> Status {
    unsafe { RedisModule_StringToDouble.unwrap()(s, len).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_set(key: *mut RedisModuleKey, s: *mut RedisModuleString) -> Status {
    unsafe { RedisModule_StringSet.unwrap()(key, s).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn replicate_verbatim(ctx: *mut RedisModuleCtx) -> Status {
    unsafe { RedisModule_ReplicateVerbatim.unwrap()(ctx).into() }
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
    unsafe { load(rdb, |rdb| RedisModule_LoadUnsigned.unwrap()(rdb)) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_signed(rdb: *mut RedisModuleIO) -> Result<i64, Error> {
    unsafe { load(rdb, |rdb| RedisModule_LoadSigned.unwrap()(rdb)) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_string(rdb: *mut RedisModuleIO) -> Result<RedisString, Error> {
    let p = unsafe { load(rdb, |rdb| RedisModule_LoadString.unwrap()(rdb))? };
    let ctx = unsafe { RedisModule_GetContextFromIO.unwrap()(rdb) };
    Ok(RedisString::from_redis_module_string(ctx, p))
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_string_buffer(rdb: *mut RedisModuleIO) -> Result<RedisBuffer, Error> {
    unsafe {
        let mut len = 0;
        let buffer = load(rdb, |rdb| {
            RedisModule_LoadStringBuffer.unwrap()(rdb, &mut len)
        })?;
        Ok(RedisBuffer::new(buffer, len))
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn replicate(ctx: *mut RedisModuleCtx, command: &str, args: &[&str]) -> Status {
    let terminated_args: Vec<RedisString> =
        args.iter().map(|s| RedisString::create(ctx, s)).collect();

    let inner_args: Vec<*mut RedisModuleString> = terminated_args.iter().map(|s| s.inner).collect();

    let cmd = CString::new(command).unwrap();

    unsafe {
        RedisModule_Replicate.unwrap()(
            ctx,
            cmd.as_ptr(),
            FMT,
            inner_args.as_ptr() as *mut c_char,
            terminated_args.len(),
        )
        .into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_double(rdb: *mut RedisModuleIO) -> Result<f64, Error> {
    unsafe { load(rdb, |rdb| RedisModule_LoadDouble.unwrap()(rdb)) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn load_float(rdb: *mut RedisModuleIO) -> Result<f32, Error> {
    unsafe { load(rdb, |rdb| RedisModule_LoadFloat.unwrap()(rdb)) }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_string(rdb: *mut RedisModuleIO, buf: &str) {
    unsafe { RedisModule_SaveStringBuffer.unwrap()(rdb, buf.as_ptr().cast::<c_char>(), buf.len()) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_double(rdb: *mut RedisModuleIO, val: f64) {
    unsafe { RedisModule_SaveDouble.unwrap()(rdb, val) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_signed(rdb: *mut RedisModuleIO, val: i64) {
    unsafe { RedisModule_SaveSigned.unwrap()(rdb, val) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_float(rdb: *mut RedisModuleIO, val: f32) {
    unsafe { RedisModule_SaveFloat.unwrap()(rdb, val) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn save_unsigned(rdb: *mut RedisModuleIO, val: u64) {
    unsafe { RedisModule_SaveUnsigned.unwrap()(rdb, val) };
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn string_append_buffer(
    ctx: *mut RedisModuleCtx,
    s: *mut RedisModuleString,
    buff: &str,
) -> Status {
    unsafe {
        RedisModule_StringAppendBuffer.unwrap()(ctx, s, buff.as_ptr() as *mut c_char, buff.len())
            .into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn subscribe_to_server_event(
    ctx: *mut RedisModuleCtx,
    event: RedisModuleEvent,
    callback: RedisModuleEventCallback,
) -> Status {
    unsafe { RedisModule_SubscribeToServerEvent.unwrap()(ctx, event, callback).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn register_info_function(ctx: *mut RedisModuleCtx, callback: RedisModuleInfoFunc) -> Status {
    unsafe { RedisModule_RegisterInfoFunc.unwrap()(ctx, callback).into() }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_section(ctx: *mut RedisModuleInfoCtx, name: Option<&str>) -> Status {
    name.map(|n| CString::new(n).unwrap()).map_or_else(
        || unsafe { RedisModule_InfoAddSection.unwrap()(ctx, ptr::null_mut()).into() },
        |n| unsafe { RedisModule_InfoAddSection.unwrap()(ctx, n.as_ptr() as *mut c_char).into() },
    )
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_str(ctx: *mut RedisModuleInfoCtx, name: &str, content: &str) -> Status {
    let name = CString::new(name).unwrap();
    let content = RedisString::create(ptr::null_mut(), content);
    unsafe {
        RedisModule_InfoAddFieldString.unwrap()(ctx, name.as_ptr() as *mut c_char, content.inner)
            .into()
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn add_info_field_long_long(
    ctx: *mut RedisModuleInfoCtx,
    name: &str,
    value: c_longlong,
) -> Status {
    let name = CString::new(name).unwrap();
    unsafe {
        RedisModule_InfoAddFieldLongLong.unwrap()(ctx, name.as_ptr() as *mut c_char, value).into()
    }
}

/// # Safety
#[cfg(feature = "experimental-api")]
pub unsafe fn export_shared_api(
    ctx: *mut RedisModuleCtx,
    func: *const ::std::os::raw::c_void,
    name: *const ::std::os::raw::c_char,
) {
    RedisModule_ExportSharedAPI.unwrap()(ctx, name, func as *mut ::std::os::raw::c_void);
}

/// # Safety
#[cfg(feature = "experimental-api")]
pub unsafe fn notify_keyspace_event(
    ctx: *mut RedisModuleCtx,
    event_type: NotifyEvent,
    event: &str,
    keyname: &RedisString,
) -> Status {
    let event = CString::new(event).unwrap();
    RedisModule_NotifyKeyspaceEvent.unwrap()(ctx, event_type.bits, event.as_ptr(), keyname.inner)
        .into()
}

#[cfg(feature = "experimental-api")]
pub fn get_keyspace_events() -> NotifyEvent {
    unsafe {
        let events = RedisModule_GetNotifyKeyspaceEvents.unwrap()();
        NotifyEvent::from_bits_truncate(events)
    }
}

#[derive(Debug, PartialEq)]
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
    unsafe { RedisModule_IsIOError.unwrap()(rdb) != 0 }
}
