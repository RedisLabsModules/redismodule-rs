// This example shows the usage of the scan functionality of the Rust Redis Module API Wrapper.
// 
// The example implements three commands:
// 
// 1. `scan_keys` - scans all keys in the database and returns their names as an array of RedisString.
// 2. `scan_key <key>` - scans all fields by using a closure and a  while loop, thus allowing an early stop. Don't use the early stop but collects all the field/value pairs as an array of RedisString.
// 3. `scan_key_foreach <key>` - scans all fields and values in a hash key using a closure that stores the field/value pairs as an array of RedisString.

use redis_module::{
    key::{KeyFlags, RedisKey}, redis_module, Context, KeysCursor, RedisError, RedisResult, RedisString, RedisValue, ScanKeyCursor
};

/// Scans all keys in the database and returns their names as an array of RedisString.
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

fn scan_key(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // only argument is the key name
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }

    let key_name = &args[1];
    let key = ctx.open_key_with_flags(key_name, KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
    let cursor  = ScanKeyCursor::new(key);

    let mut res = Vec::new();
    while cursor.scan(|_key, field, value| {
        res.push(RedisValue::BulkRedisString(field.clone()));
        res.push(RedisValue::BulkRedisString(value.clone()));
    }) {
        // here we could do something between scans if needed, like an early stop
    }

    Ok(RedisValue::Array(res))
}

/// Scans all fields and values in a hash key and returns them as an array of RedisString.
/// The command takes one argument: the name of the hash key to scan.
fn scan_key_foreach(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // only argument is the key name
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }

    let key_name = &args[1];
    let key = ctx.open_key_with_flags(key_name, KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
    let cursor  = ScanKeyCursor::new(key);
    
    let mut res = Vec::new();
    cursor.foreach(|_key, field, value| {
        res.push(RedisValue::BulkRedisString(field.clone()));
        res.push(RedisValue::BulkRedisString(value.clone()));
    });

    Ok(RedisValue::Array(res))
}


//////////////////////////////////////////////////////

redis_module! {
    name: "scan",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["scan_keys", scan_keys, "readonly", 0, 0, 0, ""],
        ["scan_key", scan_key, "readonly", 0, 0, 0, ""],
        ["scan_key_foreach", scan_key_foreach, "readonly", 0, 0, 0, ""],
    ],
}
