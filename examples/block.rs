use redis_module::{
    redis_module, Context, RedisString, RedisValue, RedisValueResult, ThreadSafeContext,
};
use std::thread;
use std::time::Duration;

fn block(ctx: &Context, _args: Vec<RedisString>) -> RedisValueResult {
    let blocked_client = ctx.block_client();

    thread::spawn(move || {
        let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);
        thread::sleep(Duration::from_millis(1000));
        thread_ctx.reply(Ok("42".into()));
    });

    // We will reply later, from the thread
    Ok(RedisValue::NoReply)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "block",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["block", block, "", 0, 0, 0],
    ],
}
