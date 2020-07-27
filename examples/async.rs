use std::os::raw::c_int;
use std::time::Duration;

use async_std::task;

#[macro_use]
extern crate redis_module;

use redis_module::{raw, Context, NextArg, RedisError, RedisResult, RedisValue, ThreadSafeContext};

pub extern "C" fn init(_: *mut raw::RedisModuleCtx) -> c_int {
    task::spawn(worker());
    0
}

// Long-lived worker thread.
async fn worker() {
    let thread_ctx = ThreadSafeContext::new();
    loop {
        {
            let ctx = thread_ctx.lock();
            ctx.log_debug("Hello from worker");
        }

        task::sleep(Duration::from_secs(30)).await;
    }
}

// Blocking command that uses async.
fn block(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let duration = args.next_i64()?;
    let duration = Duration::from_secs(duration as u64);

    let blocked_client = ctx.block_client();

    task::spawn(async move {
        let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);

        {
            let ctx = thread_ctx.lock();
            ctx.log_debug(format!("Sleeping for {:?}...", duration).as_str());
        }

        task::sleep(duration).await;

        {
            let ctx = thread_ctx.lock();
            ctx.log_debug(format!("Done sleeping for {:?}.", duration).as_str());
        }

        thread_ctx.reply(Ok("42".into()));
    });

    // We will reply later, from the thread
    Ok(RedisValue::NoReply)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "async",
    version: 1,
    data_types: [],
    init: init,
    commands: [
        ["async.block", block, "", 0, 0, 0],
    ],
}
