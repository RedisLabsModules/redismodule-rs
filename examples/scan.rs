#[macro_use]
extern crate redis_module;

use redis_module::{
    context::keys_cursor::KeysCursor, Context, RedisResult, RedisString, RedisValue,
};

fn scan_keys(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let mut keys = Vec::new();
    let cursor = KeysCursor::new();
    while cursor.scan(ctx, &|_ctx, key_name, _key| {
        keys.push(RedisValue::BulkString(
            key_name.try_as_str().unwrap().to_string(),
        ));
    }) {}
    Ok(keys.into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "scan",
    version: 1,
    data_types: [],
    commands: [
        ["SCAN_KEYS", scan_keys, "fast deny-oom readonly", 0, 0, 0],
    ],
}
