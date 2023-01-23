#[macro_use]
extern crate redis_module;

use redis_module::{server_events::FlushSubevent, Context, RedisResult, RedisString, RedisValue};
use redis_module_derive::flush_event_handler;

pub static mut NUM_FLUSHES: usize = 0;

#[flush_event_handler]
fn flushed_event_handler(_ctx: &Context, flush_event: FlushSubevent) {
    match flush_event {
        FlushSubevent::Started => unsafe { NUM_FLUSHES += 1 },
        _ => (),
    }
}

fn num_flushed(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(unsafe { NUM_FLUSHES } as i64))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "server_events",
    version: 1,
    data_types: [],
    commands: [
        ["num_flushed", num_flushed, "", 0, 0, 0],
    ],
}
