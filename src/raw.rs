// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_longlong, c_void};

extern crate libc;
extern crate enum_primitive_derive;
extern crate num_traits;

use num_traits::FromPrimitive;

use libc::size_t;

pub use crate::redisraw::bindings::*;
use crate::types::redis_log;
use crate::error::Error;

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
    fn from(v: c_int) -> Self { KeyType::from_i32(v).unwrap() }
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
    fn from(v: c_int) -> Self { ReplyType::from_i32(v).unwrap() }
}

#[derive(Primitive, Debug, PartialEq)]
pub enum Status {
    Ok = REDISMODULE_OK as isize,
    Err = REDISMODULE_ERR as isize,
}

impl From<c_int> for Status {
    fn from(v: c_int) -> Self { Status::from_i32(v).unwrap() }
}

impl From<Status> for c_int {
    fn from(s: Status) -> Self {
        s as c_int
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
) -> Result<(), &'static str> {
    let name = CString::new(name).unwrap();
    let strflags = CString::new(strflags).unwrap();

    let status: Status = unsafe {
        RedisModule_CreateCommand.unwrap()(
            ctx,
            name.as_ptr(),
            Some(cmdfunc),
            strflags.as_ptr(),
            firstkey,
            lastkey,
            keystep,
        )
    }.into();

    status.into()
}

pub fn module_init(
    ctx: *mut RedisModuleCtx,
    modulename: &str,
    module_version: c_int,
) -> Result<(), &str> {
    let modulename = CString::new(modulename.as_bytes()).unwrap();

    let status: Status = unsafe {
        Export_RedisModule_Init(
            ctx,
            modulename.as_ptr(),
            module_version,
            REDISMODULE_APIVER_1 as c_int,
        ).into()
    };

    status.into()
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

///////////////////////////////////////////////////////////////

// Helper functions for the raw bindings.

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

pub fn key_type(key: *mut RedisModuleKey) -> KeyType {
    unsafe { RedisModule_KeyType.unwrap()(key) }.into()
}

pub fn module_type_get_type(key: *mut RedisModuleKey) -> *mut RedisModuleType {
    unsafe { RedisModule_ModuleTypeGetType.unwrap()(key) }
}

pub fn module_type_get_value(key: *mut RedisModuleKey) -> *mut c_void {
    unsafe {
        RedisModule_ModuleTypeGetValue.unwrap()(key)
    }.into()
}

pub fn module_type_set_value(
    key: *mut RedisModuleKey,
    redis_type: *mut RedisModuleType,
    value: *mut c_void,
) -> Status {
    unsafe {
        RedisModule_ModuleTypeSetValue.unwrap()(
            key,
            redis_type,
            value,
        )
    }.into()
}

pub fn verify_and_get_type(
    ctx: *mut RedisModuleCtx,
    key: *mut RedisModuleKey,
    redis_type: *mut RedisModuleType,
) -> Result<KeyType, Error> {

    let key_type = key_type(key);

    // TODO: Make log() a method of the Redis and Key structs.
    redis_log(ctx, format!("key type: {:?}", key_type).as_str());

    if key_type != KeyType::Empty {
        let raw_type = module_type_get_type(key);
        if raw_type != redis_type{
            return Err(Error::generic("Key has existing value with wrong Redis type"));
        }
        redis_log(ctx, "Existing key has the correct type");
    }

    redis_log(ctx, "All OK");

    Ok(key_type)
}
