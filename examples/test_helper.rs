use redis_module::InfoContext;
use redis_module::{redis_module, Context, RedisError, RedisResult, RedisString};

fn test_helper_version(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

fn test_helper_version_rm_call(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version_rm_call()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

fn test_helper_command_name(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(ctx.current_command_name()?.into())
}

fn test_helper_err(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.is_empty() {
        return Err(RedisError::WrongArity);
    }

    let msg = args.get(1).unwrap();

    ctx.reply_error_string(msg.try_as_str().unwrap());
    Ok(().into())
}

fn test_helper_random_key(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    match ctx.random_key() {
        Some(key) => Ok(key.into()),
        None => Ok(().into()),
    }
}

fn add_info(ctx: &InfoContext, _for_crash_report: bool) {
    if let Ok(section) = ctx
        .builder()
        .add_section("test_helper")
        .field("field", "value")
    {
        let Ok(section) = section.build_section() else {
            return;
        };
        let _ = section.build_info();
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "test_helper",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    info: add_info,
    commands: [
        ["test_helper.version", test_helper_version, "", 0, 0, 0, ""],
        ["test_helper._version_rm_call", test_helper_version_rm_call, "", 0, 0, 0, ""],
        ["test_helper.name", test_helper_command_name, "", 0, 0, 0, ""],
        ["test_helper.err", test_helper_err, "", 0, 0, 0, ""],
        ["test_helper.random_key", test_helper_random_key, "", 0, 0, 0, ""],
    ],
}
