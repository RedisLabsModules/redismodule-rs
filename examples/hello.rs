#[macro_use]
extern crate redis_module;

use redis_module::InfoContext;
use redis_module::Status;

use redis_module::{Context, RedisError, RedisResult, RedisString};
fn hello_mul(_: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let nums = args
        .into_iter()
        .skip(1)
        .map(|s| s.parse_integer())
        .collect::<Result<Vec<i64>, RedisError>>()?;

    let product = nums.iter().product();

    let mut response = nums;
    response.push(product);

    Ok(response.into())
}

fn add_info(ctx: &InfoContext, _for_crash_report: bool) {
    if ctx.add_info_section(Some("hello")) == Status::Ok {
        ctx.add_info_field_str("field", "hello_value");
    }
}

//////////////////////////////////////////////////////

redis_module! {
    name: "hello",
    version: 1,
    data_types: [],
    info: add_info,
    commands: [
        ["hello.mul", hello_mul, "", 0, 0, 0],
    ],
}
