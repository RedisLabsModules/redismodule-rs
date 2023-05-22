use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use redis_module::{redis_module, Context, RedisResult, RedisString, RedisValue};
use redis_module_macros::{command, RedisValue};

#[derive(RedisValue)]
struct RedisValueDeriveInner {
    i: i64,
}

#[derive(RedisValue)]
struct RedisValueDerive {
    i: i64,
    f: f64,
    s: String,
    u: usize,
    v: Vec<i64>,
    v2: Vec<RedisValueDeriveInner>,
    hash_map: HashMap<String, String>,
    hash_set: HashSet<String>,
    ordered_map: BTreeMap<String, RedisValueDeriveInner>,
    ordered_set: BTreeSet<String>,
}

#[command(
    {
        flags: [ReadOnly, NoMandatoryKeys],
        arity: -1,
        key_spec: [
            {
                notes: "test redis value derive macro",
                flags: [ReadOnly, Access],
                begin_search: Index({ index : 0 }),
                find_keys: Range({ last_key : 0, steps : 0, limit : 0 }),
            }
        ]
    }
)]
fn redis_value_derive(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValueDerive {
        i: 10,
        f: 1.1,
        s: "s".to_owned(),
        u: 20,
        v: vec![1, 2, 3],
        v2: vec![
            RedisValueDeriveInner { i: 1 },
            RedisValueDeriveInner { i: 2 },
        ],
        hash_map: HashMap::from([("key".to_owned(), "val".to_owned())]),
        hash_set: HashSet::from(["key".to_owned()]),
        ordered_map: BTreeMap::from([("key".to_owned(), RedisValueDeriveInner { i: 10 })]),
        ordered_set: BTreeSet::from(["key".to_owned()]),
    }
    .into())
}

#[command(
    {
        flags: [ReadOnly],
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
        flags: [ReadOnly],
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
        flags: [ReadOnly, NoMandatoryKeys],
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
