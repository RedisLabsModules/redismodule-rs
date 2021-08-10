use anyhow::Context;
use anyhow::Result;
use redis::RedisError;

use utils::{get_redis_connection, start_redis_server_with_module};

mod utils;

#[test]
fn test_hello() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("hello", 6479)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6479).with_context(|| "failed to connect to redis server")?;

    let res: Vec<i32> = redis::cmd("hello.mul")
        .arg(&[3, 4])
        .query(&mut con)
        .with_context(|| "failed to run hello.mul")?;
    assert_eq!(res, vec![3, 4, 12]);

    let res: Result<Vec<i32>, RedisError> =
        redis::cmd("hello.mul").arg(&["3", "xx"]).query(&mut con);
    if let Ok(_) = res {
        return Err(anyhow::Error::msg("Should return an error"));
    }

    Ok(())
}

#[test]
fn test_keys_pos() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("keys_pos", 6480)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6480).with_context(|| "failed to connect to redis server")?;

    let res: Vec<String> = redis::cmd("keys_pos")
        .arg(&["a", "1", "b", "2"])
        .query(&mut con)
        .with_context(|| "failed to run hello.mul")?;
    assert_eq!(res, vec!["a", "b"]);

    let res: Result<Vec<String>, RedisError> =
        redis::cmd("keys_pos").arg(&["a", "1", "b"]).query(&mut con);
    if let Ok(_) = res {
        return Err(anyhow::Error::msg("Shold return an error"));
    }

    Ok(())
}

#[test]
fn test_misc_version() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("misc", 6479)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6479).with_context(|| "failed to connect to redis server")?;

    let res: Vec<i64> = redis::cmd("misc.version")
        .query(&mut con)
        .with_context(|| "failed to run misc.version")?;
    assert!(
        (res[0] == 6 && res[1] == 2 && res[2] >= 3) || (res[0] == 6 && res[1] == 0 && res[2] >= 15)
    );

    if cfg!(feature = "test") {
        // Test also the internal implementation that might not always be reached
        let res2: Vec<i64> = redis::cmd("misc._version_rm_call")
            .query(&mut con)
            .with_context(|| "failed to run misc._version_rm_call")?;
        assert_eq!(res, res2);
    }

    Ok(())
}
