#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisValue, ThreadSafeContext};
use std::thread;
use std::time::Duration;

fn block(ctx: &Context, _args: Vec<String>) -> RedisResult {
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
    data_types: [],
    commands: [
        ["block", block, "", 0, 0, 0],
    ],
}
