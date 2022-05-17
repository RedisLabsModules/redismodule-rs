#[macro_use]
extern crate redis_module;

use redis_module::{
    Context, RedisResult, RedisString, RedisValue,
    context::server_events::ServerEventData
};

static mut NUM_FLUSHES: usize = 0;
static mut NUM_ROLED_CHANGED: usize = 0;
static mut NUM_LOADINGS: usize = 0;

fn num_flushed(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(unsafe{NUM_FLUSHES} as i64))
}

fn num_roled_changed(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(unsafe{NUM_ROLED_CHANGED} as i64))
}

fn num_loading(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(unsafe{NUM_LOADINGS} as i64))
}

fn on_role_changed(_ctx: &Context, _event_data: ServerEventData) {
    let num_roled_changed = unsafe{&mut NUM_ROLED_CHANGED};
    *num_roled_changed = *num_roled_changed + 1;
}

fn on_loading_event(_ctx: &Context, _event_data: ServerEventData) {
    let num_loading = unsafe{&mut NUM_LOADINGS};
    *num_loading = *num_loading + 1;
}

fn on_flush_event(_ctx: &Context, _event_data: ServerEventData) {
    let num_flushed = unsafe{&mut NUM_FLUSHES};
    *num_flushed = *num_flushed + 1;
}

//////////////////////////////////////////////////////

redis_module! {
    name: "server_evemts",
    version: 1,
    data_types: [],
    commands: [
        ["NUM_FLUSHED", num_flushed, "fast deny-oom", 0, 0, 0],
        ["NUM_ROLED_CHANGED", num_roled_changed, "fast deny-oom", 0, 0, 0],
        ["NUM_LOADING", num_loading, "fast deny-oom", 0, 0, 0],
    ],
    server_events: [
        [@RuleChanged: on_role_changed],
        [@Loading: on_loading_event],
        [@Flush: on_flush_event],
    ]
}
