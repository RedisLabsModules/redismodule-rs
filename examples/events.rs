#[macro_use]
extern crate redis_module;

use redis_module::{Context, NotifyEvent};

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

//////////////////////////////////////////////////////

redis_module! {
    name: "events",
    version: 1,
    data_types: [],
    commands: [],
    event_handlers: [
        [@EXPIRED @EVICTED: on_event],
        [@STREAM: on_stream],
    ]
}

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {}
