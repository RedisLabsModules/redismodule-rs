#[macro_use]
extern crate redis_module;

use redis_module::{Context, KeysCursor, RedisResult, RedisString, RedisValue};

fn scan_keys(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let cursor = KeysCursor::new();
    let mut res = Vec::new();
    while cursor.scan(ctx, &|_ctx, key_name, _key| {
        res.push(RedisValue::BulkRedisString(key_name));
    }) {}
    Ok(RedisValue::Array(res))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "scan",
    version: 1,
    data_types: [],
    commands: [
        ["scan_keys", scan_keys, "readonly", 0, 0, 0],
    ],
}
