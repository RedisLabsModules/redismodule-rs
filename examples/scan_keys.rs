// This example shows the usage of the scan functionality of the Rust Redis Module API Wrapper.
//
// The example implements three commands:
//
// 1. `scan_keys` - scans all keys in the database and returns their names as an array of RedisString.
// 2. `scan_key <key>` - scans all fields by using a closure and a  while loop, thus allowing an early stop. Don't use the early stop but collects all the field/value pairs as an array of RedisString.
// 3. `scan_key_for_each <key>` - scans all fields and values in a hash key using a closure that stores the field/value pairs as an array of RedisString.
// 4. `scan_keys_for_each` - same as `scan_keys`, but using the callback based `for_each` loop.
// 5. `scan_keys_limit <n>` - scans keys until `n` of them have been collected, then stops early.
// 6. `scan_key_limit <key> <n>` - scans a hash key until `n` field/value pairs have been collected, then stops early.

use std::ops::ControlFlow;

use redis_module::{
    key::{KeyFlags, RedisKey},
    redis_module, Context, KeysCursor, RedisError, RedisResult, RedisString, RedisValue,
    ScanKeyCursor,
};

/// Scans all keys in the database and returns their names as an array of RedisString.
fn scan_keys(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let cursor = KeysCursor::new();
    let mut res = Vec::new();

    let scan_callback = |ctx: &Context, key_name: &RedisString, _key: Option<&RedisKey>| {
        res.push(RedisValue::BulkRedisString(key_name.safe_clone(ctx)));
    };

    while cursor.scan(ctx, &scan_callback) {
        // do nothing
    }
    Ok(RedisValue::Array(res))
}

/// Same as `scan_keys`, but lets the cursor drive the loop.
fn scan_keys_for_each(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let cursor = KeysCursor::new();
    let mut res = Vec::new();

    cursor.for_each(ctx, |ctx, key_name, _key| {
        res.push(RedisValue::BulkRedisString(key_name.safe_clone(ctx)));
    });

    Ok(RedisValue::Array(res))
}

/// Collects at most `n` key names and then stops scanning. Note that a break abandons the scan:
/// the keys Redis had already handed to the cursor in the current batch are skipped for good.
fn scan_keys_limit(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }
    let limit = args[1].parse_unsigned_integer()? as usize;

    let cursor = KeysCursor::new();
    let mut res = Vec::new();

    // We don't care whether we stopped early or ran out of keys, so the `ControlFlow` is dropped.
    let _ = cursor.try_for_each(ctx, |ctx, key_name, _key| {
        if res.len() >= limit {
            return ControlFlow::Break(());
        }
        res.push(RedisValue::BulkRedisString(key_name.safe_clone(ctx)));
        ControlFlow::Continue(())
    });

    Ok(RedisValue::Array(res))
}

/// Collects at most `n` field/value pairs of a hash key and then stops scanning.
fn scan_key_limit(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 3 {
        return Err(RedisError::WrongArity);
    }
    let limit = args[2].parse_unsigned_integer()? as usize;

    let key = ctx.open_key_with_flags(
        &args[1],
        KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED,
    );
    let cursor = ScanKeyCursor::new(key);

    let mut pairs = 0;
    let mut res = Vec::new();

    let _ = cursor.try_for_each(|_key, field, value| {
        if pairs >= limit {
            return ControlFlow::Break(());
        }
        res.push(RedisValue::BulkRedisString(field.clone()));
        res.push(RedisValue::BulkRedisString(value.clone()));
        pairs += 1;
        ControlFlow::Continue(())
    });

    Ok(RedisValue::Array(res))
}

fn scan_key(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // only argument is the key name
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }

    let key_name = &args[1];
    let key = ctx.open_key_with_flags(
        key_name,
        KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED,
    );
    let cursor = ScanKeyCursor::new(key);

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
fn scan_key_for_each(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // only argument is the key name
    if args.len() != 2 {
        return Err(RedisError::WrongArity);
    }

    let key_name = &args[1];
    let key = ctx.open_key_with_flags(
        key_name,
        KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED,
    );
    let cursor = ScanKeyCursor::new(key);

    let mut res = Vec::new();
    cursor.for_each(|_key, field, value| {
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
        ["scan_key_for_each", scan_key_for_each, "readonly", 0, 0, 0, ""],
        ["scan_keys_for_each", scan_keys_for_each, "readonly", 0, 0, 0, ""],
        ["scan_keys_limit", scan_keys_limit, "readonly", 0, 0, 0, ""],
        ["scan_key_limit", scan_key_limit, "readonly", 0, 0, 0, ""],
    ],
}
