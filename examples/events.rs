#[macro_use]
extern crate redis_module;

use redis_module::{Context, NotifyEvent, RedisError, RedisResult, RedisString, Status};

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

//////////////////////////////////////////////////////

redis_module! {
    name: "events",
    version: 1,
    data_types: [],
    commands: [
        ["events.send", event_send, "", 0, 0, 0],
    ],
    event_handlers: [
        [@EXPIRED @EVICTED: on_event],
        [@STREAM: on_stream],
    ]
}

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {}
