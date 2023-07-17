use redis_module::raw::KeyType;
use redis_module::{
    redis_module, Context, NextArg, RedisError, RedisResult, RedisString, RedisValue,
};

// LPOPRPUSH source destination
// Pops and returns the first element (head) of the list stored at 'source'
// and pushes the element to the last position (tail) of the list stored at
// 'destination'.
fn lpoprpush(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let src = args.next_arg()?;
    let dst = args.next_arg()?;

    let src_key = ctx.open_key_writable(&src);
    let dst_key = ctx.open_key_writable(&dst);

    let src_type = src_key.key_type();
    let dst_type = dst_key.key_type();

    if (src_type != KeyType::Empty && src_type != KeyType::List)
        || (dst_type != KeyType::Empty && dst_type != KeyType::List)
    {
        return Err(RedisError::WrongType);
    }

    match src_key.list_pop_head() {
        None => Ok(RedisValue::Null),
        Some(value) => {
            let ret_cpy = value.clone();
            dst_key.list_push_tail(value);
            Ok(RedisValue::BulkString(ret_cpy.into()))
        }
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "lists",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["LPOPRPUSH", lpoprpush, "write fast deny-oom", 1, 2, 1],
    ],
}
