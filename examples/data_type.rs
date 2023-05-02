use redis_module::native_types::RedisType;
use redis_module::{raw, redis_module, Context, NextArg, RedisResult, RedisString};
use std::os::raw::c_void;

#[derive(Debug)]
struct MyType {
    data: String,
}

static MY_REDIS_TYPE: RedisType = RedisType::new(
    "mytype123",
    0,
    raw::RedisModuleTypeMethods {
        version: raw::REDISMODULE_TYPE_METHOD_VERSION as u64,
        rdb_load: None,
        rdb_save: None,
        aof_rewrite: None,
        free: Some(free),

        // Currently unused by Redis
        mem_usage: None,
        digest: None,

        // Aux data
        aux_load: None,
        aux_save: None,
        aux_save_triggers: 0,

        free_effort: None,
        unlink: None,
        copy: None,
        defrag: None,

        copy2: None,
        free_effort2: None,
        mem_usage2: None,
        unlink2: None,
    },
);

unsafe extern "C" fn free(value: *mut c_void) {
    drop(Box::from_raw(value.cast::<MyType>()));
}

fn alloc_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_arg()?;
    let size = args.next_i64()?;

    ctx.log_debug(format!("key: {key}, size: {size}").as_str());

    let key = ctx.open_key_writable(&key);

    if let Some(value) = key.get_value::<MyType>(&MY_REDIS_TYPE)? {
        value.data = "B".repeat(size as usize);
    } else {
        let value = MyType {
            data: "A".repeat(size as usize),
        };

        key.set_value(&MY_REDIS_TYPE, value)?;
    }
    Ok(size.into())
}

fn alloc_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let key = args.next_arg()?;

    let key = ctx.open_key(&key);

    let value = match key.get_value::<MyType>(&MY_REDIS_TYPE)? {
        Some(value) => value.data.as_str().into(),
        None => ().into(),
    };

    Ok(value)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "alloc",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [
        MY_REDIS_TYPE,
    ],
    commands: [
        ["alloc.set", alloc_set, "write", 1, 1, 1],
        ["alloc.get", alloc_get, "readonly", 1, 1, 1],
    ],
}
