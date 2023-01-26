#[macro_use]
extern crate redis_module;

use std::sync::atomic::{AtomicI64, Ordering};

use redis_module::{server_events::FlushSubevent, Context, RedisResult, RedisString, RedisValue};
use redis_module_derive::flush_event_handler;

static NUM_FLUSHES: AtomicI64 = AtomicI64::new(0);

#[flush_event_handler]
fn flushed_event_handler(_ctx: &Context, flush_event: FlushSubevent) {
    if let FlushSubevent::Started = flush_event {
        NUM_FLUSHES.fetch_add(1, Ordering::SeqCst);
    }
}

fn num_flushed(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(NUM_FLUSHES.load(Ordering::SeqCst)))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "server_events",
    version: 1,
    data_types: [],
    commands: [
        ["num_flushed", num_flushed, "read-only", 0, 0, 0],
    ],
}
