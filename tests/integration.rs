mod utils;

use anyhow::Context;
use anyhow::Result;
use redis::RedisError;
use utils::{get_redis_connection, start_redis_server_with_module};

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
    if res.is_ok() {
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
        .with_context(|| "failed to run keys_pos")?;
    assert_eq!(res, vec!["a", "b"]);

    let res: Result<Vec<String>, RedisError> =
        redis::cmd("keys_pos").arg(&["a", "1", "b"]).query(&mut con);
    if res.is_ok() {
        return Err(anyhow::Error::msg("Shuold return an error"));
    }

    Ok(())
}

#[test]
fn test_hello_info() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("hello", 6481)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6481).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("INFO")
        .arg("HELLO")
        .query(&mut con)
        .with_context(|| "failed to run hello.mul")?;
    assert!(res.contains("hello_field:hello_value"));

    Ok(())
}

#[allow(unused_must_use)]
#[test]
fn test_hello_err() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("hello", 6482)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6482).with_context(|| "failed to connect to redis server")?;

    // Make sure embedded nulls do not cause a crash
    redis::cmd("hello.err")
        .arg(&["\x00\x00"])
        .query::<()>(&mut con);

    redis::cmd("hello.err")
        .arg(&["no crash\x00"])
        .query::<()>(&mut con);

    Ok(())
}
