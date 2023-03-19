#[macro_use]
extern crate redis_module;

use redis_module::raw::*;
use redis_module::{
    CallOptionsBuilder, CallReply, Context, RedisError, RedisResult, RedisString, RootCallReply,
};

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

    let call_options = CallOptionsBuilder::new().script_mode().errors_as_replies();
    let res: RootCallReply =
        ctx.call_ext::<&[&str; 0], _>("SHUTDOWN", &call_options.constract(), &[]);
    if res.get_type() != ReplyType::Error {
        return Err(RedisError::Str("Failed to set script mode on call_ext"));
    }
    let error_msg = res.get_string().unwrap();
    if !error_msg.contains("not allow") {
        return Err(RedisError::String(format!(
            "Failed to verify error messages, expected error message to contain 'not allow', error message: '{error_msg}'",
        )));
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
