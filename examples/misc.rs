#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisResult, RedisString};

fn misc_version(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

#[cfg(feature = "test")]
fn misc_version_rm_call(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let ver = ctx.get_redis_version_rm_call()?;
    let response: Vec<i64> = vec![ver.major.into(), ver.minor.into(), ver.patch.into()];

    Ok(response.into())
}

//////////////////////////////////////////////////////

#[cfg(not(feature = "test"))]
redis_module! {
    name: "misc",
    version: 1,
    data_types: [],
    commands: [
        ["misc.version", misc_version, "", 0, 0, 0],
    ],
}

#[cfg(feature = "test")]
redis_module! {
    name: "misc",
    version: 1,
    data_types: [],
    commands: [
        ["misc.version", misc_version, "", 0, 0, 0],
        ["misc._version_rm_call", misc_version_rm_call, "", 0, 0, 0],
    ],
}
