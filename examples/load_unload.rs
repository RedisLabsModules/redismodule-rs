#[macro_use]
extern crate redis_module;

use redis_module::{Context, LogLevel, RedisString, Status};

static mut GLOBAL_STATE: Option<String> = None;

fn init(ctx: &Context, args: &[RedisString]) -> Status {
    let (before, after) = unsafe {
        let before = GLOBAL_STATE.clone();
        GLOBAL_STATE.replace(format!("Args passed: {}", args.join(", ")));
        let after = GLOBAL_STATE.clone();
        (before, after)
    };
    ctx.log(
        LogLevel::Warning,
        &format!(
            "Update global state on LOAD. BEFORE: {:?}, AFTER: {:?}",
            before, after
        ),
    );

    Status::Ok
}

fn deinit(ctx: &Context) -> Status {
    let (before, after) = unsafe {
        let before = GLOBAL_STATE.take();
        let after = GLOBAL_STATE.clone();
        (before, after)
    };
    ctx.log(
        LogLevel::Warning,
        &format!(
            "Update global state on UNLOAD. BEFORE: {:?}, AFTER: {:?}",
            before, after
        ),
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
