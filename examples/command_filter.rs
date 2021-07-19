#[macro_use]
extern crate redis_module;

use redis_module::{CommandFilterContext, RedisString};
use std::time::SystemTime;

fn filter(ctx: &CommandFilterContext) {
    // Prints every command to the console
    let cmd = ctx.args_get(0).unwrap();
    eprint!("{} ", cmd);
    let count = ctx.args_count();
    for index in 1..count {
        eprint!("{} ", ctx.args_get(index).unwrap());
    }
    eprintln!("");

    // Add time field for every Hash
    if let Ok("HSET") = cmd.try_as_str() {
        ctx.args_insert(count, RedisString::create(std::ptr::null_mut(), "__TIME__"));
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        ctx.args_insert(
            count + 1,
            RedisString::create(std::ptr::null_mut(), &format!("{}", now.as_millis())),
        );
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "command_filter",
    version: 1,
    data_types: [],
    commands: [],
    filters:[
        [filter, 0],
    ]
}
