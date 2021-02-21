#[macro_use]
extern crate redis_module;

use redis_module::{Context, LogLevel, Status};

static mut GLOBAL_STATE: Option<String> = None;

fn init(ctx: &Context) -> Status {
    let (before, after) = unsafe {
        let before = GLOBAL_STATE.clone();
        GLOBAL_STATE.replace("GLOBAL DATA".to_string());
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
    data_types: [],
    init: init,
    deinit: deinit,
    commands: [],
}
