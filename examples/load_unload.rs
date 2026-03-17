use std::sync::Mutex;

use redis_module::{logging::RedisLogLevel, redis_module, Context, RedisString, Status};

static GLOBAL_STATE: Mutex<Option<String>> = Mutex::new(None);

fn init(ctx: &Context, args: &[RedisString]) -> Status {
    let mut state = GLOBAL_STATE.lock().unwrap();
    let before = state.clone();
    *state = Some(format!("Args passed: {}", args.join(", ")));
    let after = state.clone();
    ctx.log(
        RedisLogLevel::Warning,
        &format!("Update global state on LOAD. BEFORE: {before:?}, AFTER: {after:?}",),
    );

    Status::Ok
}

fn deinit(ctx: &Context) -> Status {
    let mut state = GLOBAL_STATE.lock().unwrap();
    let before = state.take();
    let after = state.clone();
    ctx.log(
        RedisLogLevel::Warning,
        &format!("Update global state on UNLOAD. BEFORE: {before:?}, AFTER: {after:?}"),
    );

    Status::Ok
}

//////////////////////////////////////////////////////

redis_module! {
    name: "load_unload",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    init: init,
    deinit: deinit,
    commands: [],
}
