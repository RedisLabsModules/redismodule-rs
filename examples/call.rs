#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString};

fn call_test(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    let res: String = ctx.call("ECHO", &["TEST"])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str("Failed calling 'ECHO TEST'"));
    }

    let res: String = ctx.call("ECHO", vec!["TEST"].as_slice())?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic str vec",
        ));
    }

    let res: String = ctx.call("ECHO", &[b"TEST"])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with static [u8]",
        ));
    }

    let res: String = ctx.call("ECHO", vec![b"TEST"].as_slice())?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: String = ctx.call("ECHO", &[&"TEST".to_string()])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str("Failed calling 'ECHO TEST' with String"));
    }

    let res: String = ctx
        .call("ECHO", vec![&"TEST".to_string()].as_slice())?
        .try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: String = ctx
        .call("ECHO", &[&ctx.create_string("TEST")])?
        .try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with RedisString",
        ));
    }

    let res: String = ctx
        .call("ECHO", vec![&ctx.create_string("TEST")].as_slice())?
        .try_into()?;
    if "TEST" != &res {
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
