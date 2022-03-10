#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString};
fn hello_mul(_: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let nums = args
        .into_iter()
        .skip(1)
        .map(|s| s.parse_integer())
        .collect::<Result<Vec<i64>, RedisError>>()?;

    let product = nums.iter().product();

    let mut response = nums;
    response.push(product);

    Ok(response.into())
}

fn encode(_: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let buffer= &args[1];
    let mut val: u64 = 0;
    for byte in buffer.as_bytes().iter() {
        val += (*byte) as u64;
    }

    Ok(val.to_string().into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "hello",
    version: 1,
    data_types: [],
    commands: [
        ["hello.mul", hello_mul, "", 0, 0, 0],
        ["hello.encode", encode, "", 0, 0, 0],
    ],
}
