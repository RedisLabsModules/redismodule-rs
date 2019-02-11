#[macro_use]
extern crate redismodule;

use std::ffi::CString;

use redismodule::{Context, Command, RedisResult, RedisError};

fn hello_mul(_: &Context, args: Vec<String>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let nums = args
        .into_iter()
        .skip(1)
        .map(parse_integer)
        .collect::<Result<Vec<i64>, RedisError>>()?;

    let product = nums.iter().product();

    let mut response = Vec::from(nums);
    response.push(product);

    return Ok(response.into());
}

fn parse_integer(arg: String) -> Result<i64, RedisError> {
    arg.parse::<i64>()
        .map_err(|_| RedisError::String(format!("Couldn't parse as integer: {}", arg)))
}

//////////////////////////////////////////////////////

const MODULE_NAME: &str = "hello";
const MODULE_VERSION: u32 = 1;

redis_module!(MODULE_NAME, MODULE_VERSION, [
    Command::new("hello.mul", hello_mul, "write"),
]);

//////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    fn run_hello_mul(args: &[&str]) -> RedisResult {
        hello_mul(
            &Context::dummy(),
            args
                .iter()
                .map(|v| String::from(*v))
                .collect(),
        )
    }

    #[test]
    fn hello_mul_valid_integer_args() {
        let result = run_hello_mul(&vec!["hello.mul", "10", "20", "30"]);

        match result {
            Ok(RedisValue::Array(v)) => {
                assert_eq!(v, vec![10, 20, 30, 6000]
                    .into_iter()
                    .map(RedisValue::Integer)
                    .collect::<Vec<_>>());
            }
            _ => assert!(false, "Bad result: {:?}", result)
        }
    }

    #[test]
    fn hello_mul_bad_integer_args() {
        let result = run_hello_mul(&vec!["hello.mul", "10", "xx", "30"]);

        match result {
            Err(RedisError::String(s)) => {
                assert_eq!(s, "Couldn't parse as integer: xx");
            }
            _ => assert!(false, "Bad result: {:?}", result)
        }
    }
}
