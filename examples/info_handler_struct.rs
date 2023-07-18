use std::collections::HashMap;

use redis_module::InfoContext;
use redis_module::{redis_module, RedisResult};
use redis_module_macros::{info_command_handler, InfoSection};

#[derive(Debug, Clone, InfoSection)]
struct Info {
    field: String,
    dictionary: HashMap<String, String>,
}

#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> RedisResult<()> {
    let mut dictionary = HashMap::new();
    dictionary.insert("key".to_owned(), "value".into());
    let data = Info {
        field: "value".to_owned(),
        dictionary,
    };
    ctx.build_one_section(data)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "info_handler_struct",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
