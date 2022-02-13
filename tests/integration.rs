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
        .with_context(|| "failed to run keys_pos")?;
    assert_eq!(res, vec!["a", "b"]);

    let res: Result<Vec<String>, RedisError> =
        redis::cmd("keys_pos").arg(&["a", "1", "b"]).query(&mut con);
    if res.is_ok() {
        return Err(anyhow::Error::msg("Should return an error"));
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

#[cfg(feature = "experimental-api")]
#[test]
fn test_command_name() -> Result<()> {
    use redis_module::{Context, Version};

    let _guards = vec![start_redis_server_with_module("test_helper", 6482)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(6482).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("test_helper.name")
        .query(&mut con)
        .with_context(|| "failed to run test_helper.name")?;

    let ctx = Context::dummy().ctx;
    //let ctx = unsafe { raw::RedisModule_GetThreadSafeContext.unwrap()(ptr::null_mut()) };

    match redis_module::Context::new(ctx).get_redis_version() {
        Ok(Version {
            major,
            minor,
            patch,
        }) => {
            if major > 6 || (major == 6 && minor > 2) || (major == 6 && minor == 2 && patch >= 5) {
                assert_eq!(res, String::from("test_helper.name"));
            } else {
                assert_eq!(
                    res,
                    String::from("API RedisModule_GetCurrentCommandName is not available")
                );
            }
        }
        Err(e) => panic!("get_redis_version failed: {}", e.to_string()),
    };

    Ok(())
}

#[test]
fn test_test_helper_info() -> Result<()> {
    let port: u16 = 6482;
    let _guards = vec![start_redis_server_with_module("test_helper", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("INFO")
        .arg("TEST_HELPER")
        .query(&mut con)
        .with_context(|| "failed to run INFO TEST_HELPER")?;
    assert!(res.contains("test_helper_field:test_helper_value"));

    Ok(())
}

#[allow(unused_must_use)]
#[test]
fn test_test_helper_err() -> Result<()> {
    let port: u16 = 6483;
    let _guards = vec![start_redis_server_with_module("hello", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    // Make sure embedded nulls do not cause a crash
    redis::cmd("test_helper.err")
        .arg(&["\x00\x00"])
        .query::<()>(&mut con);

    redis::cmd("test_helper.err")
        .arg(&["no crash\x00"])
        .query::<()>(&mut con);

    Ok(())
}
