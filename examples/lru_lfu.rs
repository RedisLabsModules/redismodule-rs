use redis_module::{
    redis_module, Context, NextArg, RedisError, RedisResult, RedisString, RedisValue,
};
use std::time::Duration;

fn set_lru_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    let lru_idle_ms = args.next_i64()?;

    let key = ctx.open_key_writable(&key_name);
    key.set_lru(Duration::from_millis(lru_idle_ms as u64))?;

    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn get_lru_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);
    match key.get_lru()? {
        Some(lru_idle) => Ok(RedisValue::Integer(lru_idle.as_millis() as i64)),
        None => Ok(RedisValue::Integer(-1)),
    }
}

fn set_lfu_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    let lfu_freq = args.next_i64()?;

    let key = ctx.open_key_writable(&key_name);
    key.set_lfu(lfu_freq)?;

    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn get_lfu_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);
    match key.get_lfu()? {
        Some(lfu_freq) => Ok(RedisValue::Integer(lfu_freq)),
        None => Ok(RedisValue::Integer(-1)),
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "lru_lfu",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["lru_lfu.set_lru", set_lru_cmd, "write fast", 1, 1, 1, ""],
        ["lru_lfu.get_lru", get_lru_cmd, "readonly fast", 1, 1, 1, ""],
        ["lru_lfu.set_lfu", set_lfu_cmd, "write fast", 1, 1, 1, ""],
        ["lru_lfu.get_lfu", get_lfu_cmd, "readonly fast", 1, 1, 1, ""],
    ],
}
