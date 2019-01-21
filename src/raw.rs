// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_longlong};

extern crate libc;

use libc::size_t;

pub use crate::redisraw::bindings::*;

bitflags! {
    pub struct KeyMode: c_int {
        const READ = REDISMODULE_READ as c_int;
        const WRITE = REDISMODULE_WRITE as c_int;
    }
}

#[derive(Debug, PartialEq)]
#[repr(i32)]
pub enum ReplyType {
    Unknown = REDISMODULE_REPLY_UNKNOWN as i32,
    String = REDISMODULE_REPLY_STRING as i32,
    Error = REDISMODULE_REPLY_ERROR as i32,
    Integer = REDISMODULE_REPLY_INTEGER as i32,
    Array = REDISMODULE_REPLY_ARRAY as i32,
    Nil = REDISMODULE_REPLY_NULL as i32,
}

// Tools that can automate this:
// https://crates.io/crates/enum_primitive
// https://crates.io/crates/num_enum
// https://crates.io/crates/enum-primitive-derive

impl From<i32> for ReplyType {
    fn from(v: i32) -> Self {
        use crate::raw::ReplyType::*;

        // TODO: Is there a way to do this with a `match`? We have different types of constants here.
        if v == Unknown as i32 {
            Unknown
        } else if v == String as i32 {
            String
        } else if v == Error as i32 {
            Error
        } else if v == Integer as i32 {
            Integer
        } else if v == Array as i32 {
            Array
        } else if v == Nil as i32 {
            Nil
        } else {
            panic!("Received unexpected reply type from Redis: {}", v)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum Status {
    Ok = REDISMODULE_OK as i32,
    Err = REDISMODULE_ERR as i32,
}

impl From<c_int> for Status {
    fn from(v: c_int) -> Self {
        // TODO: Is there a way to do this with a `match`? We have different types of constants here.
        if v == REDISMODULE_OK as c_int {
            Status::Ok
        } else if v == REDISMODULE_ERR as c_int {
            Status::Err
        } else {
            panic!("Received unexpected status from Redis: {}", v)
        }
    }
}

impl From<Status> for c_int {
    fn from(s: Status) -> Self {
        s as c_int
    }
}

pub type CommandFunc = extern "C" fn(
    ctx: *mut RedisModuleCtx,
    argv: *mut *mut RedisModuleString,
    argc: c_int,
) -> c_int;

pub fn create_command(
    ctx: *mut RedisModuleCtx,
    name: &str,
    cmdfunc: CommandFunc,
    strflags: &str,
    firstkey: i32,
    lastkey: i32,
    keystep: i32,
) -> Status {
    let name = CString::new(name).unwrap();
    let strflags = CString::new(strflags).unwrap();

    unsafe {
        RedisModule_CreateCommand.unwrap()(
            ctx,
            name.as_ptr(),
            Some(cmdfunc),
            strflags.as_ptr(),
            firstkey,
            lastkey,
            keystep,
        ).into()
    }
}

pub fn init(
    ctx: *mut RedisModuleCtx,
    modulename: &str,
    module_version: c_int,
) -> Status {
    let modulename = CString::new(modulename.as_bytes()).unwrap();
    unsafe {
        Export_RedisModule_Init(
            ctx,
            modulename.as_ptr(),
            module_version,
            REDISMODULE_APIVER_1 as c_int,
        ).into()
    }
}

// This is the one static function we need to initialize a module.
// bindgen does not generate it for us (probably since it's defined as static in redismodule.h).
#[allow(improper_ctypes)]
#[link(name = "redismodule", kind = "static")]
extern "C" {
    pub fn Export_RedisModule_Init(
        ctx: *mut RedisModuleCtx,
        modulename: *const c_char,
        module_version: c_int,
        api_version: c_int,
    ) -> c_int;
}

// Helper functions for the raw bindings.
// Taken from redis-cell.

pub fn call_reply_type(reply: *mut RedisModuleCallReply) -> ReplyType {
    unsafe {
        // TODO: Cache the unwrapped functions and use them instead of unwrapping every time?
        RedisModule_CallReplyType.unwrap()(reply).into()
    }
}

pub fn free_call_reply(reply: *mut RedisModuleCallReply) {
    unsafe {
        RedisModule_FreeCallReply.unwrap()(reply)
    }
}

pub fn call_reply_integer(reply: *mut RedisModuleCallReply) -> c_longlong {
    unsafe {
        RedisModule_CallReplyInteger.unwrap()(reply)
    }
}

pub fn call_reply_string_ptr(
    str: *mut RedisModuleCallReply,
    len: *mut size_t,
) -> *const c_char {
    unsafe {
        RedisModule_CallReplyStringPtr.unwrap()(str, len)
    }
}

pub fn close_key(kp: *mut RedisModuleKey) {
    unsafe {
        RedisModule_CloseKey.unwrap()(kp)
    }
}

pub fn create_string(
    ctx: *mut RedisModuleCtx,
    ptr: *const c_char,
    len: size_t,
) -> *mut RedisModuleString {
    unsafe { RedisModule_CreateString.unwrap()(ctx, ptr, len) }
}

pub fn free_string(ctx: *mut RedisModuleCtx, str: *mut RedisModuleString) {
    unsafe { RedisModule_FreeString.unwrap()(ctx, str) }
}

pub fn get_selected_db(ctx: *mut RedisModuleCtx) -> c_int {
    unsafe { RedisModule_GetSelectedDb.unwrap()(ctx) }
}

pub fn log(ctx: *mut RedisModuleCtx, level: *const c_char, fmt: *const c_char) {
    unsafe { RedisModule_Log.unwrap()(ctx, level, fmt) }
}

pub fn open_key(
    ctx: *mut RedisModuleCtx,
    keyname: *mut RedisModuleString,
    mode: KeyMode,
) -> *mut RedisModuleKey {
    unsafe {
        RedisModule_OpenKey.unwrap()(ctx, keyname, mode.bits) as *mut RedisModuleKey
    }
}

pub fn reply_with_array(ctx: *mut RedisModuleCtx, len: c_long) -> Status {
    unsafe { RedisModule_ReplyWithArray.unwrap()(ctx, len).into() }
}

pub fn reply_with_error(ctx: *mut RedisModuleCtx, err: *const c_char) {
    unsafe { RedisModule_ReplyWithError.unwrap()(ctx, err); }
}

pub fn reply_with_long_long(ctx: *mut RedisModuleCtx, ll: c_longlong) -> Status {
    unsafe { RedisModule_ReplyWithLongLong.unwrap()(ctx, ll).into() }
}

pub fn reply_with_string(
    ctx: *mut RedisModuleCtx,
    str: *mut RedisModuleString,
) -> Status {
    unsafe { RedisModule_ReplyWithString.unwrap()(ctx, str).into() }
}

// Sets the expiry on a key.
//
// Expire is in milliseconds.
pub fn set_expire(key: *mut RedisModuleKey, expire: c_longlong) -> Status {
    unsafe { RedisModule_SetExpire.unwrap()(key, expire).into() }
}

pub fn string_dma(
    key: *mut RedisModuleKey,
    len: *mut size_t,
    mode: KeyMode,
) -> *const c_char {
    unsafe { RedisModule_StringDMA.unwrap()(key, len, mode.bits) }
}

// Returns pointer to the C string, and sets len to its length
pub fn string_ptr_len(str: *mut RedisModuleString, len: *mut size_t) -> *const c_char {
    unsafe { RedisModule_StringPtrLen.unwrap()(str, len) }
}

pub fn string_set(key: *mut RedisModuleKey, str: *mut RedisModuleString) -> Status {
    unsafe { RedisModule_StringSet.unwrap()(key, str).into() }
}
