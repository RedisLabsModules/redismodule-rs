use redis_module::raw::{KeyType, RedisModuleStreamID};
use redis_module::{
    redis_module, Context, NextArg, RedisError, RedisString, RedisValue, RedisValueResult,
};

fn stream_read_from(ctx: &Context, args: Vec<RedisString>) -> RedisValueResult {
    let mut args = args.into_iter().skip(1);

    let stream_key = args.next_arg()?;

    let stream = ctx.open_key(&stream_key);
    let key_type = stream.key_type();

    if key_type != KeyType::Stream {
        return Err(RedisError::WrongType);
    }

    let mut iter = stream.get_stream_iterator(false)?;
    let element = iter.next();
    let id_to_keep = iter.next().as_ref().map_or_else(
        || RedisModuleStreamID {
            ms: u64::MAX,
            seq: u64::MAX,
        },
        |e| e.id,
    );

    let stream = ctx.open_key_writable(&stream_key);
    stream.trim_stream_by_id(id_to_keep, false)?;
    Ok(match element {
        Some(e) => RedisValue::BulkString(format!("{}-{}", e.id.ms, e.id.seq)),
        None => RedisValue::Null,
    })
}

//////////////////////////////////////////////////////

redis_module! {
    name: "stream",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["STREAM_POP", stream_read_from, "write", 1, 1, 1],
    ],
}
