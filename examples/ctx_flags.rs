#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisString, RedisValue};

fn role(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic(if ctx.get_flags().is_master() {"master"} else {"slave"}))
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