use redis_module::{redis_module, Context, RedisResult, RedisString, RedisValue};
use redis_module_macros::command;

#[command(
    {
        name: "classic_keys",
        flags: "readonly",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: [ReadOnly, Access],
                begin_search: Index({ index : 1 }),
                find_keys: Range({ last_key :- 1, steps : 2, limit : 0 }),
            }
        ]
    }
)]
fn classic_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic("OK"))
}

#[command(
    {
        name: "keyword_keys",
        flags: "readonly",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: [ReadOnly, Access],
                begin_search: Keyword({ keyword : "foo", startfrom : 1 }),
                find_keys: Range({ last_key :- 1, steps : 2, limit : 0 }),
            }
        ]
    }
)]
fn keyword_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::SimpleStringStatic("OK"))
}

#[command(
    {
        name: "num_keys",
        flags: "readonly no-mandatory-keys",
        arity: -2,
        key_spec: [
            {
                notes: "test command that define all the arguments at even possition as keys",
                flags: [ReadOnly, Access],
                begin_search: Index({ index : 1 }),
                find_keys: Keynum({ key_num_idx : 0, first_key : 1, key_step : 1 }),
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
