use std::{thread, time::Duration};

use redis_module::{redis_module, Context, RedisError, RedisResult, RedisString, NextArg, RedisValue, raw};
use redis_module_macros::command;


fn extract_expired_keys_from_stats(stats: RedisValue)->Result<i32, String>{
    match stats{
        RedisValue::SimpleString(s) => {
            let val = s.match_indices("expired_keys:")
                .map(|(i, _)| i)
                .last()
                .map(|i| s[i + 13..i + 14].parse::<i32>()).unwrap();
            Ok(val.ok().unwrap())
        },
        _ => Err("expired_keys is not an integer".to_string()),
    }
}

fn validate_open_key_with_no_effects(ctx: &Context, key_name: RedisString, read_write: bool) -> RedisResult {
    let key = ctx.open_key_writable(&key_name);
    let stats = ctx.call("info", &["stats"])?;
    key.set_expire(Duration::from_millis(1));
    thread::sleep(Duration::from_millis(2));
    if read_write {
        ctx.open_key_writable_with_flags(&key_name, raw::KeyMode::NOEFFECTS);
    } else {
        ctx.open_key_with_flags(&key_name, raw::KeyMode::NOEFFECTS);
    }
    let stats_after = ctx.call("info", &["stats"])?;
    let expired_before = extract_expired_keys_from_stats(stats);
    let expired_after = extract_expired_keys_from_stats(stats_after);
    if expired_before == expired_after {
        Ok(RedisValue::SimpleStringStatic("OK"))
    } else {
        Err(RedisError::String(format!("stats changed {expired_before:?} {expired_after:?}")))
    }
}

#[command(
    {
        name: "open_key_with_flags.read",
        flags: [Write, DenyOOM],
        arity: 2,
        key_spec: [
            {
                flags: [ReadOnly, Access],
                begin_search: Index({ index : 1 }),
                find_keys: Range({ last_key : 1, steps : 1, limit : 1}),
            }
        ]

    }
)]
fn read(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    validate_open_key_with_no_effects(ctx, key_name, false)
}


#[command(
    {
        name: "open_key_with_flags.write",
        flags: [Write, DenyOOM],
        arity: 2,
        key_spec: [
            {
                flags: [ReadWrite, Access],
                begin_search: Index({ index : 1 }),
                find_keys: Range({ last_key : 1, steps : 1, limit : 1}),
            }
        ]

    }
)]
fn write(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    validate_open_key_with_no_effects(ctx, key_name, true)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "open_key_with_flags",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
