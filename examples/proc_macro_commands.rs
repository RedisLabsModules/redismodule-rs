use redis_module::{redis_module, Context, RedisResult, RedisString, RedisValue};
use redis_module_macros::redis_command;

#[redis_command(
    {
        name: "classic_keys",
        flags: "readonly",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: ["RO", "ACCESS"],
                begin_search: Index(1),
                find_keys: Range((-1, 2, 0)),
            }
        ]
    }
)]
fn classic_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic("OK"))
}

#[redis_command(
    {
        name: "keyword_keys",
        flags: "readonly",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: ["RO", "ACCESS"],
                begin_search: Keyword(("foo", 1)),
                find_keys: Range((-1, 2, 0)),
            }
        ]
    }
)]
fn keyword_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic("OK"))
}

#[redis_command(
    {
        name: "num_keys",
        flags: "readonly no-mandatory-keys",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: ["RO", "ACCESS"],
                begin_search: Index(1),
                find_keys: Keynum((0, 1, 1)),
            }
        ]
    }
)]
fn num_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic("OK"))
}

redis_module! {
    name: "server_events",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [],
}
