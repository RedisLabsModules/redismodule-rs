use redis_module::{
    redis_module, redisvalue::RedisValueKey, Context, NextArg, RedisError, RedisResult,
    RedisString, RedisValue,
};
use std::collections::{HashMap, HashSet};

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
            let mut map: HashMap<RedisValueKey, RedisValue> = HashMap::with_capacity(fields.len());
            for (field, value) in values.into_iter() {
                map.insert(
                    RedisValueKey::BulkRedisString(field),
                    RedisValue::BulkRedisString(value),
                );
            }
            RedisValue::Map(map)
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
            let mut set: HashSet<RedisValueKey> = HashSet::new();
            for (_, value) in values.into_iter() {
                set.insert(RedisValueKey::BulkRedisString(value));
            }
            RedisValue::Set(set)
        }
    };

    Ok(res)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "response",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["map.mget", map_mget, "readonly", 1, 1, 1],
        ["map.unique", map_unique, "readonly", 1, 1, 1],
    ],
}
