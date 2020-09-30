#[macro_use]
extern crate redis_module;

use serde::Deserialize;

use redis_module::{de, Context, RedisError, RedisResult, RedisValue};

fn hash(ctx: &Context, _args: Vec<String>) -> RedisResult {
    // Set
    let config = ctx.open_key_writable("config");
    config.hash_set("hostname", ctx.create_string("myhost"));
    config.hash_set("port", ctx.create_string("8080"));

    // Get with old API
    let config = ctx.open_key("config");
    match config.hash_get("hostname") {
        Ok(Some(s)) => Ok(RedisValue::RedisString(s)),
        Ok(None) => Ok(RedisValue::Null),
        Err(e) => Err(e),
    }
}

fn hash_multi(ctx: &Context, _args: Vec<String>) -> RedisResult {
    #[derive(Debug, Deserialize)]
    struct Config {
        hostname: String,
        port: u32,
    }

    // Get with serde API
    let k = ctx.open_key("config");
    let c: Config = de::from_hash(&k)?;

    Ok(format!("{:?}", c).into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "hash",
    version: 1,
    data_types: [],
    commands: [
        ["hash", hash, "", 0, 0, 0],
        ["hash.multi", hash_multi, "", 0, 0, 0],
    ],
}
