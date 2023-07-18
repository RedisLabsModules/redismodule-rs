use redis_module::InfoContext;
use redis_module::{redis_module, RedisResult};
use redis_module_macros::{info_command_handler, InfoSection};

#[derive(Debug, Clone, InfoSection)]
struct InfoSection1 {
    field_1: String,
}

#[derive(Debug, Clone, InfoSection)]
struct InfoSection2 {
    field_2: String,
}

#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> RedisResult<()> {
    let data = InfoSection1 {
        field_1: "value1".to_owned(),
    };
    let _ = ctx.build_one_section(data)?;

    let data = InfoSection2 {
        field_2: "value2".to_owned(),
    };

    ctx.build_one_section(data)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "info_handler_multiple_sections",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
