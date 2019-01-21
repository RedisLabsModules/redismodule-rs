// Allow dead code in here in case I want to publish it as a crate at some
// point.
#![allow(dead_code)]

use std::ffi::CString;
use std::os::raw::{c_int, c_char};

extern crate libc;
use crate::redisraw::bindings::RedisModuleCtx;
use crate::redisraw::bindings::RedisModuleCmdFunc;
use crate::redisraw::bindings::RedisModule_CreateCommand;
use crate::redisraw::bindings::{REDISMODULE_OK, REDISMODULE_ERR};

/*
bitflags! {
    pub struct KeyMode: c_int {
        const READ = 1;
        const WRITE = (1 << 1);
    }
}
*/

#[derive(Debug, PartialEq)]
pub enum ReplyType {
    Unknown = -1,
    String = 0,
    Error = 1,
    Integer = 2,
    Array = 3,
    Nil = 4,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Status {
    Ok = REDISMODULE_OK as isize,
    Err = REDISMODULE_ERR as isize,
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
        match s {
            Status::Ok => 0,
            Status::Err => 1,
        }
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

/*
pub fn init(
    ctx: *mut RedisModuleCtx,
    modulename: &str,
    module_version: c_int,
    api_version: c_int,
) -> Status {
    let modulename = CString::new(modulename.as_bytes()).unwrap();
    unsafe {
        Export_RedisModule_Init(
            ctx,
            modulename.as_ptr(),
            module_version,
            api_version,
        ).into()
    }
}
*/

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
