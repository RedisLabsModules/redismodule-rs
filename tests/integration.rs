use anyhow::Context;
use anyhow::Result;
use redis::RedisError;

use utils::{get_redis_connection, start_redis_server_with_module};

mod utils;

#[test]
fn test_hello() -> Result<()> {
    let port: u16 = 6479;
    let _guards = vec![start_redis_server_with_module("hello", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: Vec<i32> = redis::cmd("hello.mul")
        .arg(&[3, 4])
        .query(&mut con)
        .with_context(|| "failed to run hello.mul")?;
    assert_eq!(res, vec![3, 4, 12]);

    let res: Result<Vec<i32>, RedisError> =
        redis::cmd("hello.mul").arg(&["3", "xx"]).query(&mut con);
    if res.is_ok() {
        return Err(anyhow::Error::msg("Should return an error"));
    }

    Ok(())
}

#[test]
fn test_keys_pos() -> Result<()> {
    let port: u16 = 6480;
    let _guards = vec![start_redis_server_with_module("keys_pos", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: Vec<String> = redis::cmd("keys_pos")
        .arg(&["a", "1", "b", "2"])
        .query(&mut con)
        .with_context(|| "failed to run hello.mul")?;
    assert_eq!(res, vec!["a", "b"]);

    let res: Result<Vec<String>, RedisError> =
        redis::cmd("keys_pos").arg(&["a", "1", "b"]).query(&mut con);
    if res.is_ok() {
        return Err(anyhow::Error::msg("Shold return an error"));
    }

    Ok(())
}

#[test]
fn test_test_helper_version() -> Result<()> {
    let port: u16 = 6481;
    let _guards = vec![start_redis_server_with_module("test_helper", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: Vec<i64> = redis::cmd("test_helper.version")
        .query(&mut con)
        .with_context(|| "failed to run test_helper.version")?;
    assert!(res[0] > 0);

    // Test also an internal implementation that might not always be reached
    let res2: Vec<i64> = redis::cmd("test_helper._version_rm_call")
        .query(&mut con)
        .with_context(|| "failed to run test_helper._version_rm_call")?;
    assert_eq!(res, res2);

    Ok(())
}
