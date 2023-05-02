use redis_module::{redis_module, Context, ContextFlags, RedisResult, RedisString, RedisValue};

fn role(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic(
        if ctx.get_flags().contains(ContextFlags::MASTER) {
            "master"
        } else {
            "slave"
        },
    ))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "ctx_flags",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["my_role", role, "readonly", 0, 0, 0],
    ],
}
