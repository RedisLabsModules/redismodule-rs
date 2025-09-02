use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::utils::{get_redis_connection, start_redis_server_with_module, TestConnection};
use anyhow::Context;
use anyhow::Result;
use redis::{RedisError, RedisResult, Value};
use redis_module::RedisValue;

mod utils;

#[test]
fn test_hello() -> Result<()> {
    let mut con = TestConnection::new("hello");

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
    let mut con = TestConnection::new("keys_pos");

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
fn test_helper_version() -> Result<()> {
    let mut con = TestConnection::new("test_helper");

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

#[test]
fn test_command_name() -> Result<()> {
    let mut con = TestConnection::new("test_helper");

    // Call the tested command
    let res: Result<String, RedisError> = redis::cmd("test_helper.name").query(&mut con);

    // The expected result is according to redis version
    let info: String = redis::cmd("info")
        .arg(&["server"])
        .query(&mut con)
        .with_context(|| "failed to run test_helper.name")?;

    if let Ok(ver) = redis_module::Context::version_from_info(RedisValue::SimpleString(info)) {
        if ver.major > 6
            || (ver.major == 6 && ver.minor > 2)
            || (ver.major == 6 && ver.minor == 2 && ver.patch >= 5)
        {
            assert_eq!(res.unwrap(), String::from("test_helper.name"));
        } else {
            assert!(res
                .err()
                .unwrap()
                .to_string()
                .contains("RedisModule_GetCurrentCommandName is not available"));
        }
    }

    Ok(())
}

#[test]
fn test_helper_info() -> Result<()> {
    const MODULES: [(&str, bool); 4] = [
        ("test_helper", false),
        ("info_handler_macro", false),
        ("info_handler_builder", true),
        ("info_handler_struct", true),
    ];

    MODULES
        .into_iter()
        .try_for_each(|(module, has_dictionary)| {
            let mut con = TestConnection::new(module);

            let res: String = redis::cmd("INFO")
                .arg(module)
                .query(&mut con)
                .with_context(|| format!("failed to run INFO {module}"))?;

            assert!(res.contains(&format!("{module}_field:value")));
            if has_dictionary {
                assert!(res.contains("dictionary:key=value"));
            }

            Ok(())
        })
}

#[test]
fn test_info_handler_multiple_sections() -> Result<()> {
    const MODULES: [&str; 1] = ["info_handler_multiple_sections"];

    MODULES.into_iter().try_for_each(|module| {
        let mut con = TestConnection::new(module);

        let res: String = redis::cmd("INFO")
            .arg(format!("{module}_InfoSection2"))
            .query(&mut con)
            .with_context(|| format!("failed to run INFO {module}"))?;

        assert!(res.contains(&format!("{module}_field_2:value2")));
        assert!(!res.contains(&format!("{module}_field_1:value1")));

        Ok(())
    })
}

#[allow(unused_must_use)]
#[test]
fn test_test_helper_err() -> Result<()> {
    let mut con = TestConnection::new("hello");

    // Make sure embedded nulls do not cause a crash
    redis::cmd("test_helper.err")
        .arg(&["\x00\x00"])
        .query::<()>(&mut con);

    redis::cmd("test_helper.err")
        .arg(&["no crash\x00"])
        .query::<()>(&mut con);

    Ok(())
}

#[test]
fn test_string() -> Result<()> {
    let mut con = TestConnection::new("string");

    redis::cmd("string.set")
        .arg(&["key", "value"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let res: String = redis::cmd("string.get").arg(&["key"]).query(&mut con)?;

    assert_eq!(&res, "value");

    Ok(())
}

#[test]
fn test_scan() -> Result<()> {
    let mut con = TestConnection::new("scan_keys");

    redis::cmd("set")
        .arg(&["x", "1"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    redis::cmd("set")
        .arg(&["y", "1"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let mut res: Vec<String> = redis::cmd("scan_keys").query(&mut con)?;
    res.sort();

    assert_eq!(&res, &["x", "y"]);

    Ok(())
}

#[test]
fn test_scan_key() -> Result<()> {
    let mut con = TestConnection::new("scan_keys");
    redis::cmd("hset")
        .arg(&[
            "user:123", "name", "Alice", "age", "29", "location", "Austin",
        ])
        .query::<()>(&mut con)
        .with_context(|| "failed to hset")?;

    let res: Vec<String> = redis::cmd("scan_key")
        .arg(&["user:123"])
        .query(&mut con)?;
    assert_eq!(&res, &["name", "Alice", "age", "29", "location", "Austin"]);
    Ok(())
}

#[test]
fn test_scan_key_for_each() -> Result<()> {
    let mut con = TestConnection::new("scan_keys");
    redis::cmd("hset")
        .arg(&[
            "user:123", "name", "Alice", "age", "29", "location", "Austin",
        ])
        .query::<()>(&mut con)
        .with_context(|| "failed to hset")?;

    let res: Vec<String> = redis::cmd("scan_key_foreach")
        .arg(&["user:123"])
        .query(&mut con)?;
    assert_eq!(&res, &["name", "Alice", "age", "29", "location", "Austin"]);
    Ok(())
}

#[test]
fn test_stream_reader() -> Result<()> {
    let mut con = TestConnection::new("stream");

    let _: String = redis::cmd("XADD")
        .arg(&["s", "1-1", "foo", "bar"])
        .query(&mut con)
        .with_context(|| "failed to add data to the stream")?;

    let _: String = redis::cmd("XADD")
        .arg(&["s", "1-2", "foo", "bar"])
        .query(&mut con)
        .with_context(|| "failed to add data to the stream")?;

    let res: String = redis::cmd("STREAM_POP")
        .arg(&["s"])
        .query(&mut con)
        .with_context(|| "failed to run keys_pos")?;
    assert_eq!(res, "1-1");

    let res: String = redis::cmd("STREAM_POP")
        .arg(&["s"])
        .query(&mut con)
        .with_context(|| "failed to run keys_pos")?;
    assert_eq!(res, "1-2");

    let res: usize = redis::cmd("XLEN")
        .arg(&["s"])
        .query(&mut con)
        .with_context(|| "failed to add data to the stream")?;

    assert_eq!(res, 0);

    Ok(())
}

#[test]
#[cfg(any(
    feature = "min-redis-compatibility-version-7-4",
    feature = "min-redis-compatibility-version-7-2"
))]
fn test_call() -> Result<()> {
    let mut con = TestConnection::new("call");

    let res: String = redis::cmd("call.test")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(&res, "pass");

    Ok(())
}

#[test]
fn test_ctx_flags() -> Result<()> {
    let mut con = TestConnection::new("ctx_flags");

    let res: String = redis::cmd("my_role").query(&mut con)?;

    assert_eq!(&res, "master");

    Ok(())
}

#[test]
fn test_get_current_user() -> Result<()> {
    let mut con = TestConnection::new("acl");

    let res: String = redis::cmd("get_current_user").query(&mut con)?;

    assert_eq!(&res, "default");

    Ok(())
}

#[test]
#[cfg(feature = "min-redis-compatibility-version-7-4")]
fn test_set_acl_categories() -> Result<()> {
    let mut con = TestConnection::new("acl");

    let res: Vec<String> = redis::cmd("ACL").arg("CAT").query(&mut con)?;
    assert!(res.contains(&"acl".to_owned()));

    Ok(())
}

#[test]
#[cfg(feature = "min-redis-compatibility-version-8-0")]
fn test_set_acl_categories_commands() -> Result<()> {
    let mut con = TestConnection::new("acl");

    let res: Vec<String> = redis::cmd("ACL").arg("CAT").arg("acl").query(&mut con)?;
    assert!(
        res.contains(&"verify_key_access_for_user".to_owned())
            && res.contains(&"get_current_user".to_owned())
    );

    Ok(())
}

#[test]
fn test_verify_acl_on_user() -> Result<()> {
    let mut con = TestConnection::new("acl");

    let res: String = redis::cmd("verify_key_access_for_user")
        .arg(&["default", "x"])
        .query(&mut con)?;

    assert_eq!(&res, "OK");

    let res: String = redis::cmd("ACL")
        .arg(&["SETUSER", "alice", "on", ">pass", "~cached:*", "+get"])
        .query(&mut con)?;

    assert_eq!(&res, "OK");

    let res: String = redis::cmd("verify_key_access_for_user")
        .arg(&["alice", "cached:1"])
        .query(&mut con)?;

    assert_eq!(&res, "OK");

    let res: RedisResult<String> = redis::cmd("verify_key_access_for_user")
        .arg(&["alice", "not_allow"])
        .query(&mut con);

    assert!(res.is_err());
    if let Err(res) = res {
        assert_eq!(
            res.to_string(),
            "Err: User does not have permissions on key"
        );
    }

    Ok(())
}

#[test]
fn test_key_space_notifications() -> Result<()> {
    let mut con = TestConnection::new("events");

    let res: usize = redis::cmd("events.num_key_miss").query(&mut con)?;
    assert_eq!(res, 0);

    redis::cmd("GET").arg(&["x"]).query(&mut con)?;

    let res: usize = redis::cmd("events.num_key_miss").query(&mut con)?;
    assert_eq!(res, 1);

    let _: String = redis::cmd("SET").arg(&["x", "1"]).query(&mut con)?;

    let res: String = redis::cmd("GET").arg(&["num_sets"]).query(&mut con)?;
    assert_eq!(res, "1");

    Ok(())
}

#[test]
fn test_context_mutex() -> Result<()> {
    let mut con = TestConnection::new("threads");

    let res: String = redis::cmd("set_static_data")
        .arg(&["foo"])
        .query(&mut con)?;
    assert_eq!(&res, "OK");

    let res: String = redis::cmd("get_static_data").query(&mut con)?;
    assert_eq!(&res, "foo");

    let res: String = redis::cmd("get_static_data_on_thread").query(&mut con)?;
    assert_eq!(&res, "foo");

    Ok(())
}

#[test]
fn test_server_event() -> Result<()> {
    let mut con = TestConnection::new("server_events");

    redis::cmd("flushall")
        .query(&mut con)
        .with_context(|| "failed to run flushall")?;

    let res: i64 = redis::cmd("num_flushed").query(&mut con)?;

    assert_eq!(res, 1);

    redis::cmd("flushall")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let res: i64 = redis::cmd("num_flushed").query(&mut con)?;

    assert_eq!(res, 2);

    redis::cmd("config")
        .arg(&["set", "maxmemory", "1"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let res: i64 = redis::cmd("num_max_memory_changes").query(&mut con)?;

    assert_eq!(res, 1);

    redis::cmd("config")
        .arg(&["set", "maxmemory", "0"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let res: i64 = redis::cmd("num_max_memory_changes").query(&mut con)?;

    assert_eq!(res, 2);

    let res: i64 = redis::cmd("num_crons").query(&mut con)?;

    assert!(res > 0);

    Ok(())
}

#[test]
fn test_configuration() -> Result<()> {
    let mut con = TestConnection::new("configuration");

    let config_get = |con: &mut TestConnection, config: &str| -> Result<String> {
        let res: Vec<String> = redis::cmd("config")
            .arg(&["get", config])
            .query(con)
            .with_context(|| "failed to run flushall")?;
        Ok(res[1].clone())
    };

    let config_set = |con: &mut TestConnection, config: &str, val: &str| -> Result<()> {
        let res: String = redis::cmd("config")
            .arg(&["set", config, val])
            .query(con)
            .with_context(|| "failed to run flushall")?;
        assert_eq!(res, "OK");
        Ok(())
    };

    assert_eq!(config_get(&mut con, "configuration.i64")?, "10");
    config_set(&mut con, "configuration.i64", "100")?;
    assert_eq!(config_get(&mut con, "configuration.i64")?, "100");

    assert_eq!(config_get(&mut con, "configuration.atomic_i64")?, "10");
    config_set(&mut con, "configuration.atomic_i64", "100")?;
    assert_eq!(config_get(&mut con, "configuration.atomic_i64")?, "100");

    assert_eq!(
        config_get(&mut con, "configuration.redis_string")?,
        "default"
    );
    config_set(&mut con, "configuration.redis_string", "new")?;
    assert_eq!(config_get(&mut con, "configuration.redis_string")?, "new");

    assert_eq!(config_get(&mut con, "configuration.string")?, "default");
    config_set(&mut con, "configuration.string", "new")?;
    assert_eq!(config_get(&mut con, "configuration.string")?, "new");

    assert_eq!(
        config_get(&mut con, "configuration.mutex_string")?,
        "default"
    );
    config_set(&mut con, "configuration.mutex_string", "new")?;
    assert_eq!(config_get(&mut con, "configuration.mutex_string")?, "new");

    assert_eq!(config_get(&mut con, "configuration.atomic_bool")?, "yes");
    config_set(&mut con, "configuration.atomic_bool", "no")?;
    assert_eq!(config_get(&mut con, "configuration.atomic_bool")?, "no");

    assert_eq!(config_get(&mut con, "configuration.bool")?, "yes");
    config_set(&mut con, "configuration.bool", "no")?;
    assert_eq!(config_get(&mut con, "configuration.bool")?, "no");

    assert_eq!(config_get(&mut con, "configuration.enum")?, "Val1");
    config_set(&mut con, "configuration.enum", "Val2")?;
    assert_eq!(config_get(&mut con, "configuration.enum")?, "Val2");

    assert_eq!(config_get(&mut con, "configuration.enum_mutex")?, "Val1");
    config_set(&mut con, "configuration.enum_mutex", "Val2")?;
    assert_eq!(config_get(&mut con, "configuration.enum_mutex")?, "Val2");

    let res: i64 = redis::cmd("configuration.num_changes")
        .query(&mut con)
        .with_context(|| "failed to run flushall")?;
    assert_eq!(res, 18); // the first configuration initialisation is counted as well, so we will get 18 changes.

    Ok(())
}

#[test]
fn test_response() -> Result<()> {
    let mut con = TestConnection::new("response");

    redis::cmd("hset")
        .arg(&["k", "a", "b", "c", "d", "e", "b", "f", "g"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    let mut res: Vec<String> = redis::cmd("map.mget")
        .arg(&["k", "a", "c", "e"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    res.sort();
    assert_eq!(&res, &["a", "b", "b", "c", "d", "e"]);

    let mut res: Vec<String> = redis::cmd("map.unique")
        .arg(&["k", "a", "c", "e"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    res.sort();
    assert_eq!(&res, &["b", "d"]);

    Ok(())
}

#[test]
fn test_command_proc_macro() -> Result<()> {
    let mut con = TestConnection::new("proc_macro_commands");

    let res: Vec<String> = redis::cmd("COMMAND")
        .arg(&["GETKEYS", "classic_keys", "x", "foo", "y", "bar"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(&res, &["x", "y"]);

    let res: Vec<String> = redis::cmd("COMMAND")
        .arg(&["GETKEYS", "keyword_keys", "foo", "x", "1", "y", "2"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(&res, &["x", "y"]);

    let res: Vec<String> = redis::cmd("COMMAND")
        .arg(&["GETKEYS", "num_keys", "3", "x", "y", "z", "foo", "bar"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(&res, &["x", "y", "z"]);

    let res: Vec<String> = redis::cmd("COMMAND")
        .arg(&["GETKEYS", "num_keys", "0", "foo", "bar"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert!(res.is_empty());

    Ok(())
}

#[test]
fn test_redis_value_derive() -> Result<()> {
    let mut con = TestConnection::new("proc_macro_commands");

    let res: Value = redis::cmd("redis_value_derive")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(res.as_sequence().unwrap().len(), 22);

    let res: String = redis::cmd("redis_value_derive")
        .arg(&["test"])
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(res, "OK");

    Ok(())
}

#[test]
#[cfg(any(
    feature = "min-redis-compatibility-version-7-4",
    feature = "min-redis-compatibility-version-7-2"
))]
fn test_call_blocking() -> Result<()> {
    let mut con = TestConnection::new("call");

    let res: Option<String> = redis::cmd("call.blocking")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(res, None);

    let res: Option<String> = redis::cmd("call.blocking_from_detached_ctx")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(res, None);

    Ok(())
}

#[test]
fn test_open_key_with_flags() -> Result<()> {
    let mut con = TestConnection::new("open_key_with_flags");

    // Avoid active expriation
    redis::cmd("DEBUG")
        .arg(&["SET-ACTIVE-EXPIRE", "0"])
        .query(&mut con)
        .with_context(|| "failed to run DEBUG SET-ACTIVE-EXPIRE")?;

    for cmd in ["open_key_with_flags.write", "open_key_with_flags.read"].into_iter() {
        redis::cmd("set")
            .arg(&["x", "1"])
            .query(&mut con)
            .with_context(|| "failed to run string.set")?;

        // Set experition time to 1 second.
        redis::cmd("pexpire")
            .arg(&["x", "1"])
            .query(&mut con)
            .with_context(|| "failed to run expire")?;

        // Sleep for 2 seconds, ensure expiration time has passed.
        thread::sleep(Duration::from_millis(500));

        // Open key as read only or ReadWrite with NOEFFECTS flag.
        let res = redis::cmd(cmd).arg(&["x"]).query(&mut con);
        assert_eq!(res, Ok(()));

        // Get the number of expired keys.
        let stats: String = redis::cmd("info").arg(&["stats"]).query(&mut con)?;

        // Find the number of expired keys, x,  according to the substring "expired_keys:{x}"
        let expired_keys = stats
            .match_indices("expired_keys:")
            .next()
            .map(|(i, _)| &stats[i..i + "expired_keys:".len() + 1])
            .and_then(|s| s.split(':').nth(1))
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(-1);

        // Ensure that no keys were expired.
        assert_eq!(expired_keys, 0);

        // Delete key and reset stats
        redis::cmd("del").arg(&["x"]).query(&mut con)?;
        redis::cmd("config").arg(&["RESETSTAT"]).query(&mut con)?;
    }

    Ok(())
}

#[test]
fn test_expire() -> Result<()> {
    let mut con = TestConnection::new("expire");

    // Create a key without TTL
    redis::cmd("set")
        .arg(&["key", "value"])
        .query(&mut con)
        .with_context(|| "failed to run set")?;

    let ttl: i64 = redis::cmd("ttl").arg(&["key"]).query(&mut con)?;
    assert_eq!(ttl, -1);

    // Set TTL on the key
    redis::cmd("expire.cmd")
        .arg(&["key", "100"])
        .query(&mut con)
        .with_context(|| "failed to run expire.cmd")?;

    let ttl: i64 = redis::cmd("ttl").arg(&["key"]).query(&mut con)?;
    assert!(ttl > 0);

    // Remove TTL on the key
    redis::cmd("expire.cmd")
        .arg(&["key", "-1"])
        .query(&mut con)
        .with_context(|| "failed to run expire.cmd")?;

    let ttl: i64 = redis::cmd("ttl").arg(&["key"]).query(&mut con)?;
    assert_eq!(ttl, -1);

    Ok(())
}

#[test]
fn test_defrag() -> Result<()> {
    let mut con = TestConnection::new("data_type");

    // Configure active defrag
    redis::cmd("config")
        .arg(&["set", "hz", "100"])
        .query(&mut con)
        .with_context(|| "failed to run 'config set hz 100'")?;

    redis::cmd("config")
        .arg(&["set", "active-defrag-ignore-bytes", "1"])
        .query(&mut con)
        .with_context(|| "failed to run 'config set active-defrag-ignore-bytes 1'")?;

    redis::cmd("config")
        .arg(&["set", "active-defrag-threshold-lower", "0"])
        .query(&mut con)
        .with_context(|| "failed to run 'config set active-defrag-threshold-lower 0'")?;

    redis::cmd("config")
        .arg(&["set", "active-defrag-cycle-min", "99"])
        .query(&mut con)
        .with_context(|| "failed to run 'config set active-defrag-cycle-min 99'")?;

    // enable active defrag
    if redis::cmd("config")
        .arg(&["set", "activedefrag", "yes"])
        .query::<String>(&mut con)
        .is_err()
    {
        // Server the does not support active defrag, avoid failing the test.
        return Ok(());
    }

    let start = SystemTime::now();
    loop {
        let res: HashMap<String, usize> = redis::cmd("alloc.defragstats")
            .query(&mut con)
            .with_context(|| "failed to run 'config set active-defrag-cycle-min 99'")?;
        let num_defrag_globals = res.get("num_defrag_globals").ok_or_else(|| {
            anyhow::Error::msg("Failed getting 'num_defrag_globals' value from result")
        })?;
        // Wait till we will get at least 2 defrag cycles.
        // We are looking at num_defrag_globals because this is supported by all Redis versions
        // that supports defrag.
        if *num_defrag_globals > 2 {
            break;
        }
        let duration = SystemTime::now().duration_since(start)?;
        if duration > Duration::from_secs(30) {
            return Err(anyhow::Error::msg("Failed waiting for defrag cycle"));
        }
    }

    Ok(())
}
