#[macro_use]
extern crate redis_module;

use redis_module::InfoContext;
use redis_module::Status;
use redis_module::{Context, RedisError, RedisResult, RedisString};

fn test_helper_version(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

#[cfg(feature = "test")]
fn test_helper_version_rm_call(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version_rm_call()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

fn test_helper_command_name(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(ctx.current_command_name()?.into())
}

fn test_helper_err(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let msg = args.get(1).unwrap();

    ctx.reply_error_string(msg.try_as_str().unwrap());
    Ok(().into())
}

fn add_info(ctx: &InfoContext, _for_crash_report: bool) {
    if ctx.add_info_section(Some("test_helper")) == Status::Ok {
        ctx.add_info_field_str("field", "test_helper_value");
    }
}

//////////////////////////////////////////////////////

#[cfg(feature = "test")]
redis_module! {
    name: "test_helper",
    version: 1,
    data_types: [],
    info: add_info,
    commands: [
        ["test_helper.version", test_helper_version, "", 0, 0, 0],
        ["test_helper._version_rm_call", test_helper_version_rm_call, "", 0, 0, 0],
        ["test_helper.name", test_helper_command_name, "", 0, 0, 0],
        ["test_helper.err", test_helper_err, "", 0, 0, 0],
    ],
}
