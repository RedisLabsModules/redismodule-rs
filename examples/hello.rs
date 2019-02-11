use std::iter;
use std::ffi::CString;

//#[macro_use]
extern crate redismodule;

use redismodule::{Context, Command, RedisResult, RedisValue, RedisError};

fn hello_mul(_: &Context, args: Vec<String>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut nums = vec![];

    for arg in args.into_iter().skip(1) {
        nums.push(parse_integer(&arg)?);
    }

    let product = nums.iter().product();

    let results = nums
        .into_iter()
        .chain(iter::once(product));

    return Ok(RedisValue::Array(
        results
            .map(RedisValue::Integer)
            .collect()));
}

//////////////////////////////////////////////////////

fn parse_integer(arg: &str) -> Result<i64, RedisError> {
    arg.parse::<i64>()
        .map_err(|_| RedisError::String(format!("Couldn't parse as integer: {}", arg)))
}

//////////////////////////////////////////////////////

const MODULE_NAME: &str = "hello";
const MODULE_VERSION: u32 = 1;

/*
redis_module!(MODULE_NAME, MODULE_VERSION, [
    Command::new("hello.mul", hello_mul, "write"),
]);
*/
