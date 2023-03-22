#[macro_use]
extern crate redis_module;

use lazy_static::lazy_static;
use redis_module::{
    Context, NextArg, RedisGILGuard, RedisResult, RedisString, RedisValue, ThreadSafeContext,
};
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

#[derive(Default)]
struct StaticData {
    data: String,
}

lazy_static! {
    static ref STATIC_DATA: RedisGILGuard<StaticData> = RedisGILGuard::new(StaticData::default());
}

fn set_static_data(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let val = args.next_str()?;
    let mut static_data = STATIC_DATA.lock(ctx);
    static_data.data = val.to_string();
    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn get_static_data(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let static_data = STATIC_DATA.lock(ctx);
    Ok(RedisValue::BulkString(static_data.data.clone()))
}

fn get_static_data_on_thread(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let blocked_client = ctx.block_client();
    let _ = thread::spawn(move || {
        let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);
        let ctx = thread_ctx.lock();
        let static_data = STATIC_DATA.lock(&ctx);
        thread_ctx.reply(Ok(static_data.data.clone().into()));
    });

    Ok(RedisValue::NoReply)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "threads",
    version: 1,
    data_types: [],
    commands: [
        ["threads", threads, "", 0, 0, 0],
        ["set_static_data", set_static_data, "", 0, 0, 0],
        ["get_static_data", get_static_data, "", 0, 0, 0],
        ["get_static_data_on_thread", get_static_data_on_thread, "", 0, 0, 0],
    ],
}
