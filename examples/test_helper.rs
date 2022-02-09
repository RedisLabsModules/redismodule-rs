#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisString};

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

//////////////////////////////////////////////////////

#[cfg(not(feature = "test"))]
redis_module! {
    name: "test_helper",
    version: 1,
    data_types: [],
    commands: [
        ["test_helper.version", test_helper_version, "", 0, 0, 0],
    ],
}

#[cfg(feature = "test")]
redis_module! {
    name: "test_helper",
    version: 1,
    data_types: [],
    commands: [
        ["test_helper.version", test_helper_version, "", 0, 0, 0],
        ["test_helper._version_rm_call", test_helper_version_rm_call, "", 0, 0, 0],
    ],
}
