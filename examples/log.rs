use redis_module::{redis_module, Context, RedisError, RedisResult, RedisString};

/// Logs a message that contains an interior NUL byte, the way a module logging
/// a binary-safe key or other client-controlled bytes would. Redis keys may
/// contain `\0`, which is the one byte `CString::new` rejects; before the fix
/// this panicked inside the logging call and unwound across the `extern "C"`
/// boundary, aborting the server. This command exists so the integration test
/// can prove logging is total.
fn log_with_nul(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() > 1 {
        return Err(RedisError::WrongArity);
    }

    ctx.log_notice("processing key=a\0b");
    Ok("Logged".into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "log",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["log.with_nul", log_with_nul, "", 0, 0, 0, ""],
    ],
}
