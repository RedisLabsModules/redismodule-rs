#[macro_use]
extern crate redis_module;

use bitflags::_core::time::Duration;
use redis_module::{Context, RedisError, RedisResult, ThreadSafeContext};
use std::thread;

fn threads(_: &Context, _args: Vec<String>) -> RedisResult {
    thread::spawn(move || {
        let thread_ctx = ThreadSafeContext::new();

        for _ in 0..2 {
            let ctx = thread_ctx.lock();
            ctx.call("INCR", &["threads"]).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    });

    Ok(().into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "threads",
    version: 1,
    data_types: [],
    commands: [
        ["threads", threads, "", 0, 0, 0],
    ],
}
