#[macro_use]
extern crate redis_module;

use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

fn string_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    let value = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);
    let mut dma = key.as_string_dma()?;
    dma.write(value.as_slice())
        .map(|_| RedisValue::SimpleStringStatic("OK"))
}

fn string_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);
    let res = key
        .read()?
        .map_or(RedisValue::Null, |v| RedisValue::StringBuffer(Vec::from(v)));
    Ok(res)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "string",
    version: 1,
    data_types: [],
    commands: [
        ["string.set", string_set, "write fast deny-oom", 1, 1, 1],
        ["string.get", string_get, "readonly", 1, 1, 1],
    ],
}
