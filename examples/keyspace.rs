#[macro_use]
extern crate redis_module;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

use redis_module::raw::{self, NotifyType, RedisModuleString};
use redis_module::RedisString;
use redis_module::{Context, NextArg, RedisError, RedisResult};

/// This example demonstrates subscribing to keyspace events
/// To run it:
/// cargo build --all-features --example keyspace && \
///  redis-server --loadmodule ./target/debug/examples/libkeyspace.dylib

fn del(ctx: &Context, key: &str) {
    ctx.log_debug(format!("[handle_del]: key={}", &key).as_str());
}

fn expire(ctx: &Context, key: &str) {
    ctx.log_debug(format!("[handle_expire]: key={}", &key).as_str());
}

/// Subscribe to configuration flags
/// See docs: https://github.com/antirez/redis-doc/blob/master/topics/notifications.md#configuration
/// Types `K` & `E` aren't supported using this module interface.
/// Example: `keyspace.subscribe *` will subscribe to all events and use the
/// the `router` as the callback.
fn subscribe(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);

    let notify_type = NotifyType::parse(args.next_string()?);
    ctx.log_debug(format!("notify_type: {:?}", &notify_type).as_str());

    let reply = ctx.subscribe_keyspace_events(notify_type, router);
    return Ok(reply.into());
}

// Subscribe callback
unsafe extern "C" fn router(
    ctx: *mut raw::RedisModuleCtx,
    types: c_int,
    event: *const c_char,
    key: *mut RedisModuleString,
) -> i32 {
    let event = CStr::from_ptr(event).to_str();
    let key = RedisString::from_ptr(key);
    if event.is_err() || key.is_err() {
        return 1;
    }
    let notify_type = NotifyType::from(types);
    if notify_type == NotifyType::KeyMiss {
        // drop key miss events
        return 0;
    }

    let ctx = &Context::new(ctx);
    ctx.log_debug(
        format!(
            "Keyspace event notify_type: {:?}, event: {:?}, key: {:?}",
            &notify_type, &event, &key
        )
        .as_str(),
    );
    let key = &key.unwrap();
    match event {
        Ok("del") => del(ctx, key),
        Ok("expire") => expire(ctx, key),
        _ => {}
    }
    0
}

//////////////////////////////////////////////////////

redis_module! {
    name: "keyspace",
    version: 1,
    data_types: [],
    commands: [
        ["keyspace.subscribe", subscribe, "", 0, 0, 0],
    ],
}
