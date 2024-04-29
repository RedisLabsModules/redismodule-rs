use redis_module::{redis_module, Context, NextArg, RedisError, RedisResult, RedisString};
use std::time::Duration;

fn expire_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    let ttl_sec = args.next_i64()?;
    let key = ctx.open_key_writable(&key_name);
    if ttl_sec >= 0 {
        key.set_expire(Duration::new(ttl_sec as u64, 0))
    } else {
        key.remove_expire()
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "expire",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["expire.cmd", expire_cmd, "write fast deny-oom", 1, 1, 1],
    ],
}
