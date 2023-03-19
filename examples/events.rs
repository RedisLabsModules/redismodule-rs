#[macro_use]
extern crate redis_module;

use redis_module::{Context, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};
use std::sync::atomic::{AtomicI64, Ordering};

static NUM_KEY_MISSES: AtomicI64 = AtomicI64::new(0);

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &str) {
    let msg = format!(
        "Received event: {:?} on key: {} via event: {}",
        event_type, key, event
    );
    ctx.log_debug(msg.as_str());
}

fn on_stream(ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &str) {
    ctx.log_debug("Stream event received!");
}

fn event_send(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() > 1 {
        return Err(RedisError::WrongArity);
    }

    let key_name = RedisString::create(ctx.ctx, "mykey");
    let status = ctx.notify_keyspace_event(NotifyEvent::GENERIC, "events.send", &key_name);
    match status {
        Status::Ok => Ok("Event sent".into()),
        Status::Err => Err(RedisError::Str("Generic error")),
    }
}

fn on_key_miss(_ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &str) {
    NUM_KEY_MISSES.fetch_add(1, Ordering::SeqCst);
}

fn num_key_miss(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(NUM_KEY_MISSES.load(Ordering::SeqCst)))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "events",
    version: 1,
    data_types: [],
    commands: [
        ["events.send", event_send, "", 0, 0, 0],
        ["events.num_key_miss", num_key_miss, "", 0, 0, 0],
    ],
    event_handlers: [
        [@EXPIRED @EVICTED: on_event],
        [@STREAM: on_stream],
        [@MISSED: on_key_miss],
    ]
}

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {}
