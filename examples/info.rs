#[macro_use]
extern crate redis_module;

use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};
use std::borrow::Borrow;

fn info_cmd(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let section: RedisString = args.next_arg()?;
    let field: RedisString = args.next_arg()?;

    let server_info = ctx.server_info(section.borrow());
    match server_info.field(field.borrow()) {
        None => Ok(RedisValue::Null),
        Some(v) => Ok(RedisValue::BulkRedisString(v)),
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "info",
    version: 1,
    data_types: [],
    commands: [
        ["infoex", info_cmd, "", 0, 0, 0],
    ],
}
