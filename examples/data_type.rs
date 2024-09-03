use lazy_static::lazy_static;
use libc::c_int;
use redis_module::defrag::DefragContext;
use redis_module::native_types::RedisType;
use redis_module::redisvalue::RedisValueKey;
use redis_module::{
    raw, redis_module, Context, NextArg, RedisGILGuard, RedisResult, RedisString, RedisValue,
};
use redis_module_macros::{defrag_end_function, defrag_function, defrag_start_function};
use std::os::raw::c_void;

#[derive(Debug)]
struct MyType {
    data: String,
}

lazy_static! {
    static ref NUM_KEYS_DEFRAG: RedisGILGuard<usize> = RedisGILGuard::default();
    static ref NUM_DEFRAG_START: RedisGILGuard<usize> = RedisGILGuard::default();
    static ref NUM_DEFRAG_END: RedisGILGuard<usize> = RedisGILGuard::default();
    static ref NUM_DEFRAG_GLOBALS: RedisGILGuard<usize> = RedisGILGuard::default();
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
        aux_save2: None,
        aux_save_triggers: 0,

        free_effort: None,
        unlink: None,
        copy: None,
        defrag: Some(defrag),

        copy2: None,
        free_effort2: None,
        mem_usage2: None,
        unlink2: None,
    },
);

unsafe extern "C" fn free(value: *mut c_void) {
    drop(Box::from_raw(value.cast::<MyType>()));
}

unsafe extern "C" fn defrag(
    ctx: *mut raw::RedisModuleDefragCtx,
    _key: *mut raw::RedisModuleString,
    _value: *mut *mut c_void,
) -> c_int {
    let defrag_ctx = DefragContext::new(ctx);
    let mut num_keys_defrag = NUM_KEYS_DEFRAG.lock(&defrag_ctx);
    *num_keys_defrag += 1;
    0
}

#[defrag_start_function]
fn defrag_end(defrag_ctx: &DefragContext) {
    let mut num_defrag_end = NUM_DEFRAG_END.lock(defrag_ctx);
    *num_defrag_end += 1;
}

#[defrag_end_function]
fn defrag_start(defrag_ctx: &DefragContext) {
    let mut num_defrag_start = NUM_DEFRAG_START.lock(defrag_ctx);
    *num_defrag_start += 1;
}

#[defrag_function]
fn defrag_globals(defrag_ctx: &DefragContext) {
    let mut num_defrag_globals = NUM_DEFRAG_GLOBALS.lock(defrag_ctx);
    *num_defrag_globals += 1;
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

fn alloc_defragstats(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let num_keys_defrag = NUM_KEYS_DEFRAG.lock(ctx);
    let num_defrag_globals = NUM_DEFRAG_GLOBALS.lock(ctx);
    let num_defrag_start = NUM_DEFRAG_START.lock(ctx);
    let num_defrag_end = NUM_DEFRAG_END.lock(ctx);
    Ok(RedisValue::OrderedMap(
        vec![
            (
                RedisValueKey::String("num_keys_defrag".to_owned()),
                RedisValue::Integer(*num_keys_defrag as i64),
            ),
            (
                RedisValueKey::String("num_defrag_globals".to_owned()),
                RedisValue::Integer(*num_defrag_globals as i64),
            ),
            (
                RedisValueKey::String("num_defrag_start".to_owned()),
                RedisValue::Integer(*num_defrag_start as i64),
            ),
            (
                RedisValueKey::String("num_defrag_end".to_owned()),
                RedisValue::Integer(*num_defrag_end as i64),
            ),
        ]
        .into_iter()
        .collect(),
    ))
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
        ["alloc.defragstats", alloc_defragstats, "readonly", 0, 0, 0]
    ],
}
