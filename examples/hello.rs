use std::ffi::CString;

extern crate libc;

use libc::c_int;

#[macro_use]
extern crate redismodule;

use redismodule::Context;
use redismodule::{Command, RedisResult, RedisValue, RedisError};

fn hello_mul(_: &Context, args: Vec<String>) -> RedisResult {
    if args.len() != 3 {
        return Err(RedisError::WrongArity);
    }

    let m1 = parse_integer(&args[1])?;
    let m2 = parse_integer(&args[2])?;

    let result = m1 * m2;

    return Ok(RedisValue::Array(
        vec![m1, m2, result]
            .into_iter()
            .map(|v| RedisValue::Integer(v))
            .collect()));
}

//////////////////////////////////////////////////////

fn parse_integer(arg: &str) -> Result<i64, RedisError> {
    arg.parse::<i64>()
        .map_err(|_| RedisError::String("Couldn't parse as integer"))
}

//////////////////////////////////////////////////////

const MODULE_NAME: &str = "hello";
const MODULE_VERSION: u32 = 1;

redis_module!(MODULE_NAME, MODULE_VERSION, [
    Command::new("hello.mul", hello_mul, "write"),
]);


