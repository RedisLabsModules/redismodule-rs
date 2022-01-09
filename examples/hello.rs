#[macro_use]
extern crate redis_module;

use redis_module::add_info_field_str;
use redis_module::add_info_section;
use redis_module::RedisModuleInfoCtx;
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

    let mut response = Vec::from(nums);
    response.push(product);

    return Ok(response.into());
}

fn add_info(ctx: *mut RedisModuleInfoCtx, _for_crash_report: bool) {
    if add_info_section(ctx, Some("hello")) == Status::Ok {
        add_info_field_str(ctx, "field", "hello_value");
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
