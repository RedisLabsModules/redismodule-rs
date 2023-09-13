use std::{thread, time::Duration};

use redis_module::{redis_module, Context, RedisError, RedisResult, RedisString, NextArg, RedisValue};
use redis_module::key::KeyFlag;


fn extract_expired_keys_from_stats(stats: RedisValue)->i32{
    match stats{
        RedisValue::SimpleString(s) => {
            s.match_indices("expired_keys:")
                .map(|(i, _)| i)
                .last()
                .map(|i| s[i + 13..i + 14].parse::<i32>().expect("error"))
                .expect("error")
        },
        _ => panic!("expired_keys is not an integer"),
    }
}

fn read(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);
    let stats = ctx.call("info", &["stats"]).expect("error");
    key.set_expire(Duration::from_millis(1));
    thread::sleep(Duration::from_millis(1));
    ctx.open_key_with_flags(&key_name, &[KeyFlag::NOEFFECTS].to_vec());
    let stats_after = ctx.call("info", &["stats"]).expect("error");
    let expired_before = extract_expired_keys_from_stats(stats);
    let expired_after = extract_expired_keys_from_stats(stats_after);
    let ret = expired_before == expired_after;
    match ret {
        true => Ok(RedisValue::SimpleStringStatic("OK")),
        false => Err(RedisError::String(format!("stats changed {expired_before:?} {expired_after:?}"))),
    }
}

fn write(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);
    let stats = ctx.call("info", &["stats"]).expect("error");
    key.set_expire(Duration::from_millis(1));
    thread::sleep(Duration::from_millis(1));
    ctx.open_key_writable_with_flags(&key_name, &[KeyFlag::NOEFFECTS].to_vec());
    let stats_after = ctx.call("info", &["stats"]).expect("error");
    
    let expired_before = extract_expired_keys_from_stats(stats);
    let expired_after = extract_expired_keys_from_stats(stats_after);
    let ret = expired_before == expired_after;
    match ret {
        true => Ok(RedisValue::SimpleStringStatic("OK")),
        false => Err(RedisError::String(format!("stats changed {expired_before:?} {expired_after:?}"))),
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "open_key_with_flags",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["open_key_with_flags.read", read, "write fast deny-oom", 1, 1, 1],
        ["open_key_with_flags.write", write, "write fast deny-oom", 1, 1, 1]
    ],
}
