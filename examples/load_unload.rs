#[macro_use]
extern crate redis_module;

use redis_module::{raw, Context, LogLevel};
use std::os::raw::c_int;

static mut GLOBAL_STATE: Option<String> = None;

pub extern "C" fn init(ctx: *mut raw::RedisModuleCtx) -> c_int {
    let ctx = Context::new(ctx);
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

    return raw::Status::Ok as c_int;
}

pub extern "C" fn deinit(ctx: *mut raw::RedisModuleCtx) -> c_int {
    let ctx = Context::new(ctx);
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

    raw::Status::Ok as c_int
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
