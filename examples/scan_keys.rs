// This example shows the usage of the scan functionality of the Rust Redis Module API Wrapper.
// 
// The example implements three commands:
// 
// 1. `scan_keys` - scans all keys in the database and returns their names as an array of RedisString.
// 2. `scan_key_it <key>` - scans all fields and values in a hash key providing an iterator and return the field/value pairs as an array of RedisString.
// 3. `scan_key_fe <key>` - scans all fields and values in a hash key using a closure that stores tthe field/value pairs as an array of RedisString.
// 
// `scan_key_it` always copies the field and value strings, while `scan_key_fe` uses references to the field and value strings. In that example
// both implementations need to clone the strings, because we want to return them as an array of RedisString.
//

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
    
    let res = RefCell::new(Vec::new());
    cursor.foreach(|_key, field, value| {
        let mut res = res.borrow_mut();
        res.push(RedisValue::BulkRedisString(field.clone()));
        res.push(RedisValue::BulkRedisString(value.clone()));
    });

    Ok(RedisValue::Array(res.take()))
}

/// Scans all fields and values in a hash key and returns them as an array of RedisString.
/// The command takes one argument: the name of the hash key to scan.
fn scan_key_iterator(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // only argument is the key name
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }

    let key_name = &args[1];
    let mut res = Vec::new();
    let key = ctx.open_key_with_flags(key_name, KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
    let cursor  = ScanKeyCursor::new(key);
    
    for (no, (field, value)) in cursor.iter().enumerate() {
        res.push(RedisValue::BulkRedisString(field));
        res.push(RedisValue::BulkRedisString(value));
    }
    
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
        ["scan_key_it", scan_key_iterator, "readonly", 0, 0, 0, ""],
        ["scan_key_fe", scan_key_foreach, "readonly", 0, 0, 0, ""],
    ],
}
