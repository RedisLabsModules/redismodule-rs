use redis_module::{redis_module, RedisResult};
use redis_module::{InfoContext, Status};
use redis_module_macros::info_command_handler;

// The deprecated methods are allowed since this is just an example.
#[allow(deprecated)]
#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> RedisResult<()> {
    if ctx.add_info_section(Some("info")) == Status::Ok {
        ctx.add_info_field_str("field", "value");
    }

    Ok(())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "info_handler_macro",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
