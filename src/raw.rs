// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

extern crate enum_primitive_derive;
extern crate libc;
extern crate num_traits;

use bitflags::bitflags;
use enum_primitive_derive::Primitive;
use libc::size_t;
use num_traits::FromPrimitive;
use std::ffi::CString;
use std::os::raw::{c_char, c_double, c_int, c_long, c_longlong};
use std::ptr;
use std::slice;

pub use crate::redisraw::bindings::*;
use crate::RedisString;
use crate::{RedisBuffer, RedisError};

bitflags! {
    pub struct KeyMode: c_int {
        const READ = REDISMODULE_READ as c_int;
        const WRITE = REDISMODULE_WRITE as c_int;
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
}

impl From<c_int> for KeyType {
    fn from(v: c_int) -> Self {
        KeyType::from_i32(v).unwrap()
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
        ReplyType::from_i32(v).unwrap()
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
        Status::from_i32(v).unwrap()
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

pub const FMT: *const c_char = b"v\0".as_ptr() as *const c_char;

// REDISMODULE_HASH_DELETE is defined explicitly here because bindgen cannot
// parse typecasts in C macro constants yet.
// See https://github.com/rust-lang/rust-bindgen/issues/316
pub const REDISMODULE_HASH_DELETE: *const RedisModuleString = 1 as *const RedisModuleString;

// Helper functions for the raw bindings.

pub fn call_reply_type(reply: *mut RedisModuleCallReply) -> ReplyType {
    unsafe {
        // TODO: Cache the unwrapped functions and use them instead of unwrapping every time?
        RedisModule_CallReplyType.unwrap()(reply).into()
    }
}

pub fn free_call_reply(reply: *mut RedisModuleCallReply) {
    unsafe { RedisModule_FreeCallReply.unwrap()(reply) }
}

pub fn call_reply_integer(reply: *mut RedisModuleCallReply) -> c_longlong {
    unsafe { RedisModule_CallReplyInteger.unwrap()(reply) }
}

pub fn call_reply_array_element(
    reply: *mut RedisModuleCallReply,
    idx: usize,
) -> *mut RedisModuleCallReply {
    unsafe { RedisModule_CallReplyArrayElement.unwrap()(reply, idx) }
}

pub fn call_reply_length(reply: *mut RedisModuleCallReply) -> usize {
    unsafe { RedisModule_CallReplyLength.unwrap()(reply) }
}

pub fn call_reply_string_ptr(reply: *mut RedisModuleCallReply, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_CallReplyStringPtr.unwrap()(reply, len) }
}

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

pub fn close_key(kp: *mut RedisModuleKey) {
    unsafe { RedisModule_CloseKey.unwrap()(kp) }
}

pub fn open_key(
    ctx: *mut RedisModuleCtx,
    keyname: *mut RedisModuleString,
    mode: KeyMode,
) -> *mut RedisModuleKey {
    unsafe { RedisModule_OpenKey.unwrap()(ctx, keyname, mode.bits) as *mut RedisModuleKey }
}

pub fn reply_with_array(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    unsafe { RedisModule_ReplyWithArray.unwrap()(ctx, len).into() }
}

pub fn reply_with_error(ctx: *mut RedisModuleCtx, err: *const c_char) {
    unsafe {
        RedisModule_ReplyWithError.unwrap()(ctx, err);
    }
}

pub fn reply_with_long_long(ctx: *mut RedisModuleCtx, ll: c_longlong) -> Status {
    unsafe { RedisModule_ReplyWithLongLong.unwrap()(ctx, ll).into() }
}

pub fn reply_with_double(ctx: *mut RedisModuleCtx, f: c_double) -> Status {
    unsafe { RedisModule_ReplyWithDouble.unwrap()(ctx, f).into() }
}

pub fn reply_with_string(ctx: *mut RedisModuleCtx, s: *mut RedisModuleString) -> Status {
    unsafe { RedisModule_ReplyWithString.unwrap()(ctx, s).into() }
}

// Sets the expiry on a key.
//
// Expire is in milliseconds.
pub fn set_expire(key: *mut RedisModuleKey, expire: c_longlong) -> Status {
    unsafe { RedisModule_SetExpire.unwrap()(key, expire).into() }
}

pub fn string_dma(key: *mut RedisModuleKey, len: *mut size_t, mode: KeyMode) -> *const c_char {
    unsafe { RedisModule_StringDMA.unwrap()(key, len, mode.bits) }
}

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
        _ => Err(RedisError::Str("ERR key is not a hash value")),
    }
}

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

pub fn list_push(
    key: *mut RedisModuleKey,
    list_where: Where,
    element: *mut RedisModuleString,
) -> Status {
    unsafe { RedisModule_ListPush.unwrap()(key, list_where as i32, element).into() }
}

pub fn list_pop(key: *mut RedisModuleKey, list_where: Where) -> *mut RedisModuleString {
    unsafe { RedisModule_ListPop.unwrap()(key, list_where as i32) }
}

// Returns pointer to the C string, and sets len to its length
pub fn string_ptr_len(s: *mut RedisModuleString, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_StringPtrLen.unwrap()(s, len) }
}

pub fn string_set(key: *mut RedisModuleKey, s: *mut RedisModuleString) -> Status {
    unsafe { RedisModule_StringSet.unwrap()(key, s).into() }
}

pub fn replicate_verbatim(ctx: *mut RedisModuleCtx) -> Status {
    unsafe { RedisModule_ReplicateVerbatim.unwrap()(ctx).into() }
}

pub fn load_unsigned(rdb: *mut RedisModuleIO) -> u64 {
    unsafe { RedisModule_LoadUnsigned.unwrap()(rdb) }
}

pub fn load_signed(rdb: *mut RedisModuleIO) -> i64 {
    unsafe { RedisModule_LoadSigned.unwrap()(rdb) }
}

pub fn load_string(rdb: *mut RedisModuleIO) -> String {
    let p = unsafe { RedisModule_LoadString.unwrap()(rdb) };
    RedisString::from_ptr(p)
        .expect("UTF8 encoding error in load string")
        .to_string()
}

pub fn load_string_buffer(rdb: *mut RedisModuleIO) -> RedisBuffer {
    unsafe {
        let mut len = 0;
        let buffer = RedisModule_LoadStringBuffer.unwrap()(rdb, &mut len);
        RedisBuffer::new(buffer, len)
    }
}

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

pub fn load_double(rdb: *mut RedisModuleIO) -> f64 {
    unsafe { RedisModule_LoadDouble.unwrap()(rdb) }
}

pub fn load_float(rdb: *mut RedisModuleIO) -> f32 {
    unsafe { RedisModule_LoadFloat.unwrap()(rdb) }
}

pub fn save_string(rdb: *mut RedisModuleIO, buf: &str) {
    unsafe { RedisModule_SaveStringBuffer.unwrap()(rdb, buf.as_ptr() as *const c_char, buf.len()) };
}

pub fn save_double(rdb: *mut RedisModuleIO, val: f64) {
    unsafe { RedisModule_SaveDouble.unwrap()(rdb, val) };
}

pub fn save_signed(rdb: *mut RedisModuleIO, val: i64) {
    unsafe { RedisModule_SaveSigned.unwrap()(rdb, val) };
}

pub fn save_float(rdb: *mut RedisModuleIO, val: f32) {
    unsafe { RedisModule_SaveFloat.unwrap()(rdb, val) };
}

pub fn save_unsigned(rdb: *mut RedisModuleIO, val: u64) {
    unsafe { RedisModule_SaveUnsigned.unwrap()(rdb, val) };
}

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

pub fn subscribe_to_server_event(
    ctx: *mut RedisModuleCtx,
    event: RedisModuleEvent,
    callback: RedisModuleEventCallback,
) -> Status {
    unsafe { RedisModule_SubscribeToServerEvent.unwrap()(ctx, event, callback).into() }
}

#[cfg(feature = "experimental-api")]
pub fn export_shared_api(
    ctx: *mut RedisModuleCtx,
    func: *const ::std::os::raw::c_void,
    name: *const ::std::os::raw::c_char,
) {
    unsafe { RedisModule_ExportSharedAPI.unwrap()(ctx, name, func as *mut ::std::os::raw::c_void) };
}

#[cfg(feature = "experimental-api")]
pub fn notify_keyspace_event(
    ctx: *mut RedisModuleCtx,
    event_type: NotifyEvent,
    event: &str,
    keyname: &str,
) -> Status {
    let event = CString::new(event).unwrap();
    let keyname = RedisString::create(ctx, keyname);
    unsafe {
        RedisModule_NotifyKeyspaceEvent.unwrap()(
            ctx,
            event_type.bits,
            event.as_ptr(),
            keyname.inner,
        )
        .into()
    }
}

#[cfg(feature = "experimental-api")]
pub fn get_keyspace_events() -> NotifyEvent {
    unsafe {
        let events = RedisModule_GetNotifyKeyspaceEvents.unwrap()();
        NotifyEvent::from_bits_truncate(events)
    }
}
