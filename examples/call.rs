#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString};

fn call_test(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    let res: Result<String, RedisError> = ctx.call("ECHO", &["TEST"])?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str("Failed calling 'ECHO TEST'"));
    }

    let res: Result<String, RedisError> = ctx.call("ECHO", vec!["TEST"].as_slice())?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic str vec",
        ));
    }

    let res: Result<String, RedisError> = ctx.call("ECHO", &[b"TEST"])?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with static [u8]",
        ));
    }

    let res: Result<String, RedisError> = ctx.call("ECHO", vec![b"TEST"].as_slice())?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: Result<String, RedisError> = ctx.call("ECHO", &[&"TEST".to_string()])?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str("Failed calling 'ECHO TEST' with String"));
    }

    let res: Result<String, RedisError> = ctx
        .call("ECHO", vec![&"TEST".to_string()].as_slice())?
        .into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: Result<String, RedisError> = ctx.call("ECHO", &[&ctx.create_string("TEST")])?.into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with RedisString",
        ));
    }

    let res: Result<String, RedisError> = ctx
        .call("ECHO", vec![&ctx.create_string("TEST")].as_slice())?
        .into();
    if "TEST" != &res? {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with dynamic array of RedisString",
        ));
    }

    Ok("pass".into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "call",
    version: 1,
    data_types: [],
    commands: [
        ["call.test", call_test, "", 0, 0, 0],
    ],
}
