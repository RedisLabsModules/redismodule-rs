// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_int, c_char};

extern crate libc;

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

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(i32)]
pub enum Status {
    Ok = REDISMODULE_OK as i32,
    Err = REDISMODULE_ERR as i32,
}

impl From<c_int> for Status {
    fn from(v: c_int) -> Self {
        match v {
            0 => Status::Ok,
            1 => Status::Err,
            _ => panic!("Received unexpected status from Redis: {}", v),
        }
    }
}

impl From<Status> for c_int {
    fn from(s: Status) -> Self {
        s as c_int
    }
}

pub fn create_command(
    ctx: *mut RedisModuleCtx,
    name: &str,
    cmdfunc: RedisModuleCmdFunc,
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
            cmdfunc,
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
