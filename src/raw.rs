// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

use std::os::raw::{c_char, c_double, c_int, c_long, c_longlong};

extern crate enum_primitive_derive;
extern crate libc;
extern crate num_traits;

use libc::size_t;
use num_traits::FromPrimitive;
use std::ptr::null_mut;
use std::slice;

pub use crate::redisraw::bindings::*;
use crate::RedisBuffer;
use crate::RedisString;

bitflags! {
    pub struct KeyMode: c_int {
        const READ = REDISMODULE_READ as c_int;
        const WRITE = REDISMODULE_WRITE as c_int;
    }
}

#[derive(Primitive, Debug, PartialEq)]
pub enum KeyType {
    Empty = REDISMODULE_KEYTYPE_EMPTY as isize,
    String = REDISMODULE_KEYTYPE_STRING as isize,
    List = REDISMODULE_KEYTYPE_LIST as isize,
    Hash = REDISMODULE_KEYTYPE_HASH as isize,
    Set = REDISMODULE_KEYTYPE_SET as isize,
    ZSet = REDISMODULE_KEYTYPE_ZSET as isize,
    Module = REDISMODULE_KEYTYPE_MODULE as isize,
}

impl From<c_int> for KeyType {
    fn from(v: c_int) -> Self {
        KeyType::from_i32(v).unwrap()
    }
}

// This hack is required since derive(Primitive) requires all values to have the same type,
// and REDISMODULE_REPLY_UNKNOWN is i32 while the rest are u32.
// Casting to isize inside the enum definition breaks the derive(Primitive) macro.
const REDISMODULE_REPLY_UNKNOWN_ISIZE: isize = REDISMODULE_REPLY_UNKNOWN as isize;
const REDISMODULE_REPLY_STRING_ISIZE: isize = REDISMODULE_REPLY_STRING as isize;
const REDISMODULE_REPLY_ERROR_ISIZE: isize = REDISMODULE_REPLY_ERROR as isize;
const REDISMODULE_REPLY_INTEGER_ISIZE: isize = REDISMODULE_REPLY_INTEGER as isize;
const REDISMODULE_REPLY_ARRAY_ISIZE: isize = REDISMODULE_REPLY_ARRAY as isize;
const REDISMODULE_REPLY_NULL_ISIZE: isize = REDISMODULE_REPLY_NULL as isize;

#[derive(Primitive, Debug, PartialEq)]
pub enum ReplyType {
    Unknown = REDISMODULE_REPLY_UNKNOWN_ISIZE,
    String = REDISMODULE_REPLY_STRING_ISIZE,
    Error = REDISMODULE_REPLY_ERROR_ISIZE,
    Integer = REDISMODULE_REPLY_INTEGER_ISIZE,
    Array = REDISMODULE_REPLY_ARRAY_ISIZE,
    Nil = REDISMODULE_REPLY_NULL_ISIZE,
}

impl From<c_int> for ReplyType {
    fn from(v: c_int) -> Self {
        ReplyType::from_i32(v).unwrap()
    }
}

#[derive(Primitive, Debug, PartialEq)]
pub enum Status {
    Ok = REDISMODULE_OK as isize,
    Err = REDISMODULE_ERR as isize,
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

pub fn call_reply_string_ptr(reply: *mut RedisModuleCallReply, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_CallReplyStringPtr.unwrap()(reply, len) }
}

pub fn call_reply_string(reply: *mut RedisModuleCallReply) -> String {
    unsafe {
        let len: *mut size_t = null_mut();
        let str: *mut u8 = RedisModule_CallReplyStringPtr.unwrap()(reply, len) as *mut u8;
        String::from_utf8(
            slice::from_raw_parts(str, *len)
                .into_iter()
                .map(|v| *v)
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

pub fn load_double(rdb: *mut RedisModuleIO) -> f64 {
    unsafe { RedisModule_LoadDouble.unwrap()(rdb) }
}

pub fn load_float(rdb: *mut RedisModuleIO) -> f32 {
    unsafe { RedisModule_LoadFloat.unwrap()(rdb) }
}

pub fn save_string(rdb: *mut RedisModuleIO, buf: &String) {
    unsafe { RedisModule_SaveStringBuffer.unwrap()(rdb, buf.as_ptr() as *mut i8, buf.len()) };
}
