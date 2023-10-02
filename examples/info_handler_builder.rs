use redis_module::info_command_handler;
use redis_module::InfoContext;
use redis_module::{redis_module, RedisResult};

#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> RedisResult<()> {
    ctx.builder()
        .add_section("info")
        .field("field", "value")?
        .add_dictionary("dictionary")
        .field("key", "value")?
        .build_dictionary()?
        .build_section()?
        .build_info()?;

    Ok(())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "info_handler_builder",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
