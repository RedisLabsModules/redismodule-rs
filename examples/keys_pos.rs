#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString, RedisValue};

fn keys_pos(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // Number of args (excluding command name) must be even
    if (args.len() - 1) % 2 != 0 {
        return Err(RedisError::WrongArity);
    }

    if ctx.is_keys_position_request() {
        for i in 1..args.len() {
            if (i - 1) % 2 == 0 {
                ctx.key_at_pos(i as i32);
            }
        }
        return Ok(RedisValue::NoReply);
    }

    let reply: Vec<_> = args.iter().skip(1).step_by(2).collect();

    Ok(reply.into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "keys_pos",
    version: 1,
    data_types: [],
    commands: [
        ["keys_pos", keys_pos, "getkeys-api", 1, 1, 1],
    ],
}
