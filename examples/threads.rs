#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisString, ThreadSafeContext};
use std::mem::drop;
use std::thread;
use std::time::Duration;

fn threads(_: &Context, _args: Vec<RedisString>) -> RedisResult {
    thread::spawn(move || {
        let thread_ctx = ThreadSafeContext::new();

        loop {
            let ctx = thread_ctx.lock();
            ctx.call("INCR", &["threads"]).unwrap();
            // release the lock as soon as we're done accessing redis memory
            drop(ctx);
            thread::sleep(Duration::from_millis(1000));
        }
    });

    Ok(().into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "threads",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["threads", threads, "", 0, 0, 0],
    ],
}
