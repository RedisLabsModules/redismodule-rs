use redis_module::{
    redis_module, redisvalue::RedisValueKey, Context, NextArg, RedisError, RedisResult,
    RedisString, RedisValue,
};
use std::collections::{BTreeMap, BTreeSet};

fn map_mget(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let fields: Vec<RedisString> = args.collect();

    let key = ctx.open_key(&key_name);
    let values = key.hash_get_multi(&fields)?;
    let res = match values {
        None => RedisValue::Null,
        Some(values) => {
            let mut map: BTreeMap<RedisValueKey, RedisValue> = BTreeMap::new();
            for (field, value) in values.into_iter() {
                map.insert(
                    RedisValueKey::BulkRedisString(field),
                    RedisValue::BulkRedisString(value),
                );
            }
            RedisValue::OrderedMap(map)
        }
    };

    Ok(res)
}

fn map_unique(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;

    let fields: Vec<RedisString> = args.collect();

    let key = ctx.open_key(&key_name);
    let values = key.hash_get_multi(&fields)?;
    let res = match values {
        None => RedisValue::Null,
        Some(values) => {
            let mut set: BTreeSet<RedisValueKey> = BTreeSet::new();
            for (_, value) in values.into_iter() {
                set.insert(RedisValueKey::BulkRedisString(value));
            }
            RedisValue::OrderedSet(set)
        }
    };

    Ok(res)
}

/// `map.hget key field`: one field through [`RedisKey::hash_get_by_string`], which takes the
/// field as the `RedisString` it already is instead of copying it into a C string.
fn map_hget(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);
    let key_name = args.next_arg()?;
    let field = args.next_arg()?;

    let key = ctx.open_key(&key_name);
    Ok(match key.hash_get_by_string(&field)? {
        None => RedisValue::Null,
        Some(value) => RedisValue::BulkRedisString(value),
    })
}

//////////////////////////////////////////////////////

redis_module! {
    name: "response",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["map.mget", map_mget, "readonly", 1, 1, 1, ""],
        ["map.unique", map_unique, "readonly", 1, 1, 1, ""],
        ["map.hget", map_hget, "readonly", 1, 1, 1, ""],
    ],
}
