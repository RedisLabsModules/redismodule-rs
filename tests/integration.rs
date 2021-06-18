mod utils;

use anyhow::Result;
use redis::RedisError;
use utils::{get_redis_connection, start_redis_server_with_module};

#[test]
fn test_hello() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("hello", 6479)?];
    let mut con = get_redis_connection(6479)?;

    let res: Vec<i32> = redis::cmd("hello.mul").arg(&[3, 4]).query(&mut con)?;
    assert_eq!(res, vec![3, 4, 12]);

    let res: Result<Vec<i32>, RedisError> =
        redis::cmd("hello.mul").arg(&["3", "xx"]).query(&mut con);
    if let Ok(_) = res {
        return Err(anyhow::Error::msg("Shold return an error"));
    }

    Ok(())
}

#[test]
fn test_keys_pos() -> Result<()> {
    let _guards = vec![start_redis_server_with_module("keys_pos", 6480)?];
    let mut con = get_redis_connection(6480)?;

    let res: Vec<String> = redis::cmd("keys_pos")
        .arg(&["a", "1", "b", "2"])
        .query(&mut con)?;
    assert_eq!(res, vec!["a", "b"]);

    let res: Result<Vec<String>, RedisError> =
        redis::cmd("keys_pos").arg(&["a", "1", "b"]).query(&mut con);
    if let Ok(_) = res {
        return Err(anyhow::Error::msg("Shold return an error"));
    }

    Ok(())
}
