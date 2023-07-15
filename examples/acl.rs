use std::sync::Mutex;

use lazy_static::{__Deref, lazy_static};
use redis_module::{
    redis_module, AclPermissions, Context, NextArg, RedisError, RedisResult, RedisString,
    RedisUser, RedisValue, Status,
};

lazy_static! {
    static ref USER: Mutex<RedisUser> = Mutex::new(RedisUser::new("acl"));
}

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

fn authenticate_with_user(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let user = USER.lock()?;
    ctx.authenticate_client_with_user(user.deref())?;
    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn init(_ctx: &Context, _args: &[RedisString]) -> Status {
    // Set the user ACL
    let _ = USER.lock().unwrap().set_acl("on allcommands allkeys");

    // Module initialized
    Status::Ok
}

//////////////////////////////////////////////////////

redis_module! {
    name: "acl",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    init: init,
    commands: [
        ["authenticate_with_user", authenticate_with_user, "", 0, 0, 0],
        ["verify_key_access_for_user", verify_key_access_for_user, "", 0, 0, 0],
        ["get_current_user", get_current_user, "", 0, 0, 0],
    ],
}
