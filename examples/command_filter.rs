#[macro_use]
extern crate redis_module;

use std::time::{SystemTime};
use redis_module::{RedisString, CommandFilterContext};

fn filter(ctx: &CommandFilterContext) {

    // Prints every command to the console
    let cmd = ctx.args_get(0);
    eprint!("{} ", cmd);
    let count = ctx.args_count();
    for index in 1..count {
        eprint!("{} ", ctx.args_get(index));
    }
    eprintln!("");


    // Add time field for every Hash 
    if let Ok("HSET") = cmd.try_as_str() {
        ctx.args_insert(count, RedisString::create(std::ptr::null_mut(), "__TIME__"));
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        ctx.args_insert(count+1, RedisString::create(std::ptr::null_mut(), &format!("{}", now.as_millis())));
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
