#[macro_use]
extern crate redis_module;

use redis_module::{key::RedisKey, Context, KeysCursor, RedisResult, RedisString, RedisValue};

fn scan_keys(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let cursor = KeysCursor::new();
    let mut res = Vec::new();

    let scan_callback = |_ctx: &Context, key_name: RedisString, _key: Option<&RedisKey>| {
        res.push(RedisValue::BulkRedisString(key_name));
    };

    while cursor.scan(ctx, &scan_callback) {
        // do nothing
    }
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
