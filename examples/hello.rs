use std::ffi::CString;

//#[macro_use]
extern crate redismodule;

use redismodule::{Context, Command, RedisResult, RedisValue, RedisError};

fn hello_mul(_: &Context, args: Vec<String>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let maybe_nums: Vec<_> = args
        .into_iter()
        .skip(1) // The command itself
        .map(parse_integer)
        .collect();

    let mut nums = vec![];
    for n in maybe_nums {
        nums.push(n?);
    }

    nums.push(nums.iter().product());

    return Ok(RedisValue::Array(nums
        .into_iter()
        .map(RedisValue::Integer)
        .collect()));
}

//////////////////////////////////////////////////////

fn parse_integer(arg: String) -> Result<i64, RedisError> {
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
