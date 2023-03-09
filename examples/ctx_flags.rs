#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisString, RedisValue, ContextFlags};

fn role(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic(
        if ctx.get_flags().contains(ContextFlags::MASTER) {
            "master"
        } else {
            "slave"
        },
    ))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "ctx_flags",
    version: 1,
    data_types: [],
    commands: [
        ["my_role", role, "readonly", 0, 0, 0],
    ],
}
