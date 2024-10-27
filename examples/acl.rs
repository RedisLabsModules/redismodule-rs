use redis_module::{
    redis_module, AclPermissions, Context, NextArg, RedisError, RedisResult, RedisString,
    RedisValue,
};

fn verify_key_access_for_user(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let user = args.next_arg()?;
    let key = args.next_arg()?;
    let res = ctx.acl_check_key_permission(&user, &key, &AclPermissions::all());
    if let Err(err) = res {
        return Err(RedisError::String(format!("Err {err}")));
    }
    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn get_current_user(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::BulkRedisString(ctx.get_current_user()))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "acl",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["verify_key_access_for_user", verify_key_access_for_user, "", 0, 0, 0, ""],
        ["get_current_user", get_current_user, "", 0, 0, 0, ""],
    ],
}
