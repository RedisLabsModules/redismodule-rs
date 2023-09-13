use crate::utils::{get_redis_connection, start_redis_server_with_module};
use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{RedisError, RedisResult};

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
fn test_helper_version() -> Result<()> {
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

#[test]
fn test_command_name() -> Result<()> {
    use redis_module::RedisValue;

    let port: u16 = 6482;
    let _guards = vec![start_redis_server_with_module("test_helper", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
            let port: u16 = 6483;
            let _guards = vec![start_redis_server_with_module(module, port)
                .with_context(|| "failed to start redis server")?];
            let mut con =
                get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
        let port: u16 = 6500;
        let _guards = vec![start_redis_server_with_module(module, port)
            .with_context(|| "failed to start redis server")?];
        let mut con =
            get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6484;
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

#[test]
fn test_string() -> Result<()> {
    let port: u16 = 6485;
    let _guards = vec![start_redis_server_with_module("string", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6486;
    let _guards = vec![start_redis_server_with_module("scan_keys", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
fn test_stream_reader() -> Result<()> {
    let port: u16 = 6487;
    let _guards = vec![start_redis_server_with_module("stream", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
fn test_call() -> Result<()> {
    let port: u16 = 6488;
    let _guards = vec![start_redis_server_with_module("call", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("call.test")
        .query(&mut con)
        .with_context(|| "failed to run string.set")?;

    assert_eq!(&res, "pass");

    Ok(())
}

#[test]
fn test_ctx_flags() -> Result<()> {
    let port: u16 = 6489;
    let _guards = vec![start_redis_server_with_module("ctx_flags", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("my_role").query(&mut con)?;

    assert_eq!(&res, "master");

    Ok(())
}

#[test]
fn test_get_current_user() -> Result<()> {
    let port: u16 = 6490;
    let _guards = vec![start_redis_server_with_module("acl", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: String = redis::cmd("get_current_user").query(&mut con)?;

    assert_eq!(&res, "default");

    Ok(())
}

#[test]
fn test_verify_acl_on_user() -> Result<()> {
    let port: u16 = 6491;
    let _guards = vec![start_redis_server_with_module("acl", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6492;
    let _guards = vec![start_redis_server_with_module("events", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res: usize = redis::cmd("events.num_key_miss").query(&mut con)?;
    assert_eq!(res, 0);

    let _ = redis::cmd("GET").arg(&["x"]).query(&mut con)?;

    let res: usize = redis::cmd("events.num_key_miss").query(&mut con)?;
    assert_eq!(res, 1);

    let _: String = redis::cmd("SET").arg(&["x", "1"]).query(&mut con)?;

    let res: String = redis::cmd("GET").arg(&["num_sets"]).query(&mut con)?;
    assert_eq!(res, "1");

    Ok(())
}

#[test]
fn test_context_mutex() -> Result<()> {
    let port: u16 = 6493;
    let _guards = vec![start_redis_server_with_module("threads", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6494;
    let _guards = vec![start_redis_server_with_module("server_events", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6495;
    let _guards = vec![start_redis_server_with_module("configuration", port)
        .with_context(|| "failed to start redis server")?];

    let config_get = |config: &str| -> Result<String> {
        let mut con =
            get_redis_connection(port).with_context(|| "failed to connect to redis server")?;
        let res: Vec<String> = redis::cmd("config")
            .arg(&["get", config])
            .query(&mut con)
            .with_context(|| "failed to run flushall")?;
        Ok(res[1].clone())
    };

    let config_set = |config: &str, val: &str| -> Result<()> {
        let mut con =
            get_redis_connection(port).with_context(|| "failed to connect to redis server")?;
        let res: String = redis::cmd("config")
            .arg(&["set", config, val])
            .query(&mut con)
            .with_context(|| "failed to run flushall")?;
        assert_eq!(res, "OK");
        Ok(())
    };

    assert_eq!(config_get("configuration.i64")?, "10");
    config_set("configuration.i64", "100")?;
    assert_eq!(config_get("configuration.i64")?, "100");

    assert_eq!(config_get("configuration.atomic_i64")?, "10");
    config_set("configuration.atomic_i64", "100")?;
    assert_eq!(config_get("configuration.atomic_i64")?, "100");

    assert_eq!(config_get("configuration.redis_string")?, "default");
    config_set("configuration.redis_string", "new")?;
    assert_eq!(config_get("configuration.redis_string")?, "new");

    assert_eq!(config_get("configuration.string")?, "default");
    config_set("configuration.string", "new")?;
    assert_eq!(config_get("configuration.string")?, "new");

    assert_eq!(config_get("configuration.mutex_string")?, "default");
    config_set("configuration.mutex_string", "new")?;
    assert_eq!(config_get("configuration.mutex_string")?, "new");

    assert_eq!(config_get("configuration.atomic_bool")?, "yes");
    config_set("configuration.atomic_bool", "no")?;
    assert_eq!(config_get("configuration.atomic_bool")?, "no");

    assert_eq!(config_get("configuration.bool")?, "yes");
    config_set("configuration.bool", "no")?;
    assert_eq!(config_get("configuration.bool")?, "no");

    assert_eq!(config_get("configuration.enum")?, "Val1");
    config_set("configuration.enum", "Val2")?;
    assert_eq!(config_get("configuration.enum")?, "Val2");

    assert_eq!(config_get("configuration.enum_mutex")?, "Val1");
    config_set("configuration.enum_mutex", "Val2")?;
    assert_eq!(config_get("configuration.enum_mutex")?, "Val2");

    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;
    let res: i64 = redis::cmd("configuration.num_changes")
        .query(&mut con)
        .with_context(|| "failed to run flushall")?;
    assert_eq!(res, 18); // the first configuration initialisation is counted as well, so we will get 18 changes.

    Ok(())
}

#[test]
fn test_response() -> Result<()> {
    let port: u16 = 6496;
    let _guards = vec![start_redis_server_with_module("response", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6497;
    let _guards = vec![start_redis_server_with_module("proc_macro_commands", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6498;
    let _guards = vec![start_redis_server_with_module("proc_macro_commands", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
fn test_call_blocking() -> Result<()> {
    let port: u16 = 6499;
    let _guards = vec![start_redis_server_with_module("call", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
        get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

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
    let port: u16 = 6501;
    let _guards = vec![start_redis_server_with_module("open_key_with_flags", port)
        .with_context(|| "failed to start redis server")?];
    let mut con =
    get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

    let res = redis::cmd("open_key_with_flags.read").arg(&["x"]).query(&mut con);
    assert_eq!(res, Ok(()));

    let res = redis::cmd("open_key_with_flags.write").arg(&["x"]).query(&mut con);
    assert_eq!(res, Ok(()));

    Ok(())
}
