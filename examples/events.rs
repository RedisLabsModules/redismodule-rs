#[macro_use]
extern crate redis_module;

use redis_module::apierror::{APIResult};
use redis_module::{
    Context, NotifyEvent, RedisError, RedisResult, RedisString, RedisValue, Status,
};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI64, Ordering};

static NUM_KEY_MISSES: AtomicI64 = AtomicI64::new(0);

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    if key == b"num_sets" {
        // break infinit look
        return;
    }
    let msg = format!(
        "Received event: {:?} on key: {} via event: {}",
        event_type,
        std::str::from_utf8(key).unwrap(),
        event
    );
    ctx.log_notice(msg.as_str());
    let _: APIResult<Status> = ctx.add_post_notification_job(|ctx| {
        // it is not safe to write inside the notification callback itself.
        // So we perform the write on a post job notificaiton.
        if let Err(e) = ctx.call("incr", &["num_sets"]) {
            ctx.log_warning(&format!("Error on incr command, {}.", e));
        }
    });
}

fn on_stream(ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &[u8]) {
    ctx.log_debug("Stream event received!");
}

fn event_send(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() > 1 {
        return Err(RedisError::WrongArity);
    }

    let key_name = RedisString::create(NonNull::new(ctx.ctx), "mykey");
    let status = ctx.notify_keyspace_event(NotifyEvent::GENERIC, "events.send", &key_name);
    match status {
        Status::Ok => Ok("Event sent".into()),
        Status::Err => Err(RedisError::Str("Generic error")),
    }
}

fn on_key_miss(_ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &[u8]) {
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
        [@STRING: on_event],
        [@STREAM: on_stream],
        [@MISSED: on_key_miss],
    ],
}

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {}
