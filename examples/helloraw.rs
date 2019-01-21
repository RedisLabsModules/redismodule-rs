use std::ffi::CString;

extern crate libc;

use std::os::raw::{c_int, c_char};

extern crate redismodule;

//use redismodule::raw;
use redismodule::raw::RedisModuleCtx;
use redismodule::raw::RedisModuleString;
use redismodule::raw::REDISMODULE_APIVER_1;
use redismodule::raw::RedisModule_CreateCommand;
use redismodule::raw::RedisModule_ReplyWithLongLong;
use redismodule::raw::Export_RedisModule_Init;
use redismodule::raw::Status;
use redismodule::raw::init;
use redismodule::raw::create_command;
//use redismodule::raw::Status;

const MODULE_NAME: &str = "helloraw";
const MODULE_VERSION: c_int = 1;


#[allow(non_snake_case)]
#[no_mangle]
// TODO: This symbol doesn't need to be external (only RedisModule_OnLoad does)
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

    Status::Ok.into()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut RedisModuleCtx,
    argv: *mut *mut RedisModuleString,
    argc: c_int,
) -> c_int {

    // Init
    let modulename = MODULE_NAME;
    let module_version = MODULE_VERSION;

    if init(ctx, modulename, module_version) == Status::Err {
        return Status::Err.into();
    }

    // Create command
    let name = "helloraw";
    let cmdfunc = Some(Hello_RedisCommand);
    let strflags = "write";
    let firstkey = 1;
    let lastkey = 1;
    let keystep = 1;

    if create_command(ctx, name, cmdfunc, strflags, firstkey, lastkey, keystep) == Status::Err {
        return Status::Err.into();
    }

    Status::Ok.into()
}

