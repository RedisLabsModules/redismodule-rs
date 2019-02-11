#[macro_use]
extern crate redismodule;

use redismodule::{Context, Command, RedisResult, NextArg};
use redismodule::native_types::RedisType;

#[derive(Debug)]
struct MyType {
    data: String,
}

static MY_REDIS_TYPE: RedisType = RedisType::new("mytype123");

fn alloc_set(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;
    let size = args.next_i64()?;

    ctx.log_debug(format!("key: {}, size: {}", key, size).as_str());

    let key = ctx.open_key_writable(&key);

    if key.is_empty() {
        let value = MyType {
            data: "A".repeat(size as usize)
        };

        ctx.log_debug(format!("key is empty; setting to value: '{:?}'", value).as_str());

        key.set_value(&MY_REDIS_TYPE, value)?;
    } else {
        ctx.log_debug(format!("key exists; getting value").as_str());

        let value: &mut MyType = key.get_value(&MY_REDIS_TYPE)?;
        ctx.log_debug(format!("got value: '{:?}'", value).as_str());

        value.data = "B".repeat(size as usize);
        ctx.log_debug(format!("new value: '{:?}'", value).as_str());
    };

    Ok(size.into())
}


/*
fn alloc_get(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;

    let key = ctx.open_key(&key);

    key.verify_and_get_type(&MY_REDIS_TYPE)?;
    let my = key.get_value() as *mut MyType;

    if my.is_null() {
        r.reply_integer(0)?;
        return Ok(());
    }

    let my = unsafe { &mut *my };
    let size = my.data.len();

    r.reply_array(2)?;
    r.reply_integer(size as i64)?;
    r.reply_string(my.data.as_str())?;

    Ok(())
}
*/

//////////////////////////////////////////////////////

const MODULE_NAME: &str = "alloc";
const MODULE_VERSION: c_int = 1;

redis_module!(
    MODULE_NAME,
    MODULE_VERSION,
    [
        &MY_REDIS_TYPE,
    ],
    [
        Command::new("alloc.set", alloc_set, "write"),
    ]
);
