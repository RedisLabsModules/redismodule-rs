#[macro_use]
extern crate redismodule;

use redismodule::{Context, RedisResult, NextArg};
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

    let value = match key.get_value::<MyType>(&MY_REDIS_TYPE)? {
        None => ().into(),
        Some(value) => {
            // TODO: Use the value
            let _ = value;
            "some value".into()
        }
    };

    Ok(value)
}

//////////////////////////////////////////////////////

redis_module!{
    name: "alloc",
    version: 1,
    data_types: [
        MY_REDIS_TYPE,
    ],
    commands: [
        ["alloc.set", alloc_set, "write"],
        ["alloc.get", alloc_get, ""],
    ],
}
