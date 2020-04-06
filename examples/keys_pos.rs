#[macro_use]
extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisValue};

fn keys_pos(ctx: &Context, args: Vec<String>) -> RedisResult {
    // Number of args (excluding command name) must be even
    if (args.len() - 1) % 2 != 0 {
        return Err(RedisError::WrongArity);
    }

    if ctx.is_keys_position_request() {
        for i in 1..args.len() {
            if (i - 1) % 2 == 0 {
                ctx.key_at_pos(i as i32);
            }
        }
        return Ok(RedisValue::NoReply);
    }

    let reply: Vec<_> = args.iter().skip(1).step_by(2).collect();

    return Ok(reply.into());
}

//////////////////////////////////////////////////////

redis_module! {
    name: "keys_pos",
    version: 1,
    data_types: [],
    commands: [
        ["keys_pos", keys_pos, "getkeys-api", 1, 1, 1],
    ],
}

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;
    use redis_module::RedisValue;

    fn into_string_vec(args: &[&str]) -> Vec<String> {
        args.iter().map(|v| String::from(*v)).collect()
    }

    fn into_redisvalue_vec(args: &[&str]) -> Vec<RedisValue> {
        args.iter()
            .map(|&s| RedisValue::BulkString(s.to_string()))
            .collect()
    }

    #[test]
    fn test_keys_pos() {
        let result = keys_pos(
            &Context::dummy(),
            into_string_vec(&["keys_pos", "a", "1", "b", "2"]),
        );

        match result {
            Ok(RedisValue::Array(v)) => {
                assert_eq!(v, into_redisvalue_vec(&["a", "b"]));
            }
            _ => assert!(false, "Bad result: {:?}", result),
        }
    }

    #[test]
    fn test_keys_pos_bad_args() {
        let result = keys_pos(
            &Context::dummy(),
            into_string_vec(&["keys_pos", "a", "1", "b"]),
        );

        match result {
            Err(RedisError::WrongArity) => (),
            _ => assert!(false, "Bad result: {:?}", result),
        }
    }
}
