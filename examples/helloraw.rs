use std::ffi::CString;

extern crate libc;

use std::os::raw::{c_int, c_char};

extern crate redismodule;

//use redismodule::raw;
use redismodule::redisraw::bindings::RedisModuleCtx;
use redismodule::redisraw::bindings::RedisModuleString;
use redismodule::redisraw::bindings::{REDISMODULE_OK, REDISMODULE_ERR, REDISMODULE_APIVER_1};
use redismodule::redisraw::bindings::RedisModule_CreateCommand;
use redismodule::redisraw::bindings::RedisModule_ReplyWithLongLong;
use redismodule::raw::Export_RedisModule_Init;
//use redismodule::raw::Status;

const MODULE_NAME: &str = "helloraw";
const MODULE_VERSION: c_int = 1;


#[allow(non_snake_case)]
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn Hello_RedisCommand(
    ctx: *mut RedisModuleCtx,
    argv: *mut *mut RedisModuleString,
    argc: c_int,
) -> c_int {

    // TODO: Handle return value
    unsafe {
        RedisModule_ReplyWithLongLong.unwrap()(
            ctx,
            42,
        );
    }

    REDISMODULE_OK as c_int
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut RedisModuleCtx,
    argv: *mut *mut RedisModuleString,
    argc: c_int,
) -> c_int {

    // Init
    let modulename = MODULE_NAME;
    let module_version = MODULE_VERSION;

    let modulename = CString::new(modulename.as_bytes()).unwrap();
    if unsafe {
        Export_RedisModule_Init(
            ctx,
            modulename.as_ptr(),
            module_version,
            REDISMODULE_APIVER_1 as c_int,
        )
    } == REDISMODULE_ERR as c_int {
        return REDISMODULE_ERR as c_int
    }

    // Create command
    let name = "helloraw";
    let strflags = "write";
    let firstkey = 1;
    let lastkey = 1;
    let keystep = 1;

    let name = CString::new(name).unwrap();
    let strflags = CString::new(strflags).unwrap();
    let cmdfunc = Hello_RedisCommand;
    if unsafe {
        RedisModule_CreateCommand.unwrap()(
            ctx,
            name.as_ptr(),
            Some(cmdfunc),
            strflags.as_ptr(),
            firstkey,
            lastkey,
            keystep,
        )
    } == REDISMODULE_ERR as c_int {
        return REDISMODULE_ERR as c_int
    }

    REDISMODULE_OK as c_int
}


