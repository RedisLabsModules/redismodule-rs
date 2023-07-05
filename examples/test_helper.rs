use std::collections::HashMap;

use redis_module::InfoContext;
use redis_module::RedisResult;
use redis_module::{redis_module, Context, RedisError, RedisString, RedisValueResult};
use redis_module_macros::info_command_handler;
use redis_module_macros::InfoSection;

fn test_helper_version(ctx: &Context, _args: Vec<RedisString>) -> RedisValueResult {
    let ver = ctx.get_redis_version()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

fn test_helper_version_rm_call(ctx: &Context, _args: Vec<RedisString>) -> RedisValueResult {
    let ver = ctx.get_redis_version_rm_call()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

fn test_helper_command_name(ctx: &Context, _args: Vec<RedisString>) -> RedisValueResult {
    Ok(ctx.current_command_name()?.into())
}

fn test_helper_err(ctx: &Context, args: Vec<RedisString>) -> RedisValueResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let msg = args.get(1).unwrap();

    ctx.reply_error_string(msg.try_as_str().unwrap());
    Ok(().into())
}

#[derive(Debug, Clone, InfoSection)]
struct InfoData {
    field: String,
    dictionary: HashMap<String, String>,
}

#[info_command_handler]
fn add_info(ctx: &InfoContext, _for_crash_report: bool) -> RedisResult {
    let mut dictionary = HashMap::new();
    dictionary.insert("key".to_owned(), "value".into());
    let data = InfoData {
        field: "test_helper_value".to_owned(),
        dictionary,
    };
    ctx.build_from(data)
}

//////////////////////////////////////////////////////

redis_module! {
    name: "test_helper",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["test_helper.version", test_helper_version, "", 0, 0, 0],
        ["test_helper._version_rm_call", test_helper_version_rm_call, "", 0, 0, 0],
        ["test_helper.name", test_helper_command_name, "", 0, 0, 0],
        ["test_helper.err", test_helper_err, "", 0, 0, 0],
    ],
}
