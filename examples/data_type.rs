#[macro_use]
extern crate redismodule;

use redismodule::{Context, Command, RedisResult, NextArg};
use redismodule::native_types::RedisType;
use redismodule::redismodule::RedisValue;

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

    match key.get_value::<MyType>(&MY_REDIS_TYPE)? {
        None => {
            let value = MyType {
                data: "A".repeat(size as usize)
            };

            key.set_value(&MY_REDIS_TYPE, value)?;
        }
        Some(value) => {
            value.data = "B".repeat(size as usize);
        }
    }

    Ok(size.into())
}

fn alloc_get(ctx: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_string()?;

    let key = ctx.open_key_writable(&key); // TODO: Use read-only key

    match key.get_value::<MyType>(&MY_REDIS_TYPE)? {
        None => Ok(RedisValue::None),
        Some(value) => {
            // TODO: Use the value
            let _ = value;
            Ok("some value".into())
        }
    }

}

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
