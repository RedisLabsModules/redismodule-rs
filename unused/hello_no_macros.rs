use std::ffi::CString;

extern crate libc;

use libc::c_int;

extern crate redis_module;

use redis_module::raw;
use redis_module::Context;
use redis_module::{Command, RedisResult, RedisValue, RedisError};

const MODULE_NAME: &str = "hello";
const MODULE_VERSION: u32 = 1;


fn hello_mul(_: &Context, args: Vec<String>) -> RedisResult {
    if args.len() != 3 {
        return Err(RedisError::WrongArity);
    }

    // TODO: Write generic RedisValue::parse method
    if let RedisValue::Integer(m1) = parse_integer(&args[1])? {
        if let RedisValue::Integer(m2) = parse_integer(&args[2])? {
            let result = m1 * m2;

            return Ok(RedisValue::Array(
                vec![m1, m2, result]
                    .into_iter()
                    .map(|v| RedisValue::Integer(v))
                    .collect()));
        }
    }

    Err(RedisError::String("Something went wrong"))
}

//////////////////////////////////////////////////////

#[no_mangle]
#[allow(non_snake_case)]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut raw::RedisModuleCtx,
    _argv: *mut *mut raw::RedisModuleString,
    _argc: c_int,
) -> c_int {
    unsafe {
        //////////////////

        let module_name = MODULE_NAME;
        let module_version = MODULE_VERSION;

        let commands = [
            Command::new("hello.mul", hello_mul, "write"),
        ];

        //////////////////

        let module_name = CString::new(module_name).unwrap();
        let module_version = module_version as c_int;

        if raw::Export_RedisModule_Init(
            ctx,
            module_name.as_ptr(),
            module_version,
            raw::REDISMODULE_APIVER_1 as c_int,
        ) == raw::Status::Err as _ { return raw::Status::Err as _; }

        for command in &commands {
            let name = CString::new(command.name).unwrap();
            let flags = CString::new(command.flags).unwrap();
            let (firstkey, lastkey, keystep) = (1, 1, 1);

            if raw::RedisModule_CreateCommand.unwrap()(
                ctx,
                name.as_ptr(),
                command.wrap_handler(),
                flags.as_ptr(),
                firstkey, lastkey, keystep,
            ) == raw::Status::Err as _ { return raw::Status::Err as _; }
        }

        raw::Status::Ok as _
    }
}

fn parse_integer(arg: &str) -> RedisResult {
    arg.parse::<i64>()
        .map_err(|_| RedisError::String("Couldn't parse as integer"))
        .map(|v| RedisValue::Integer(v))
    //Error::generic(format!("Couldn't parse as integer: {}", arg).as_str()))
}
