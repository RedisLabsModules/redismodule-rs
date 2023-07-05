use redis_module::{redis_module, Context, NextArg, RedisString, RedisValueResult};
use std::time::Duration;

fn callback(ctx: &Context, data: String) {
    ctx.log_debug(format!("[callback]: {}", data).as_str());
}

type MyData = String;

fn timer_create(ctx: &Context, args: Vec<RedisString>) -> RedisValueResult {
    let mut args = args.into_iter().skip(1);
    let duration = args.next_i64()?;
    let data: MyData = args.next_string()?;

    let timer_id = ctx.create_timer(Duration::from_millis(duration as u64), callback, data);

    return Ok(format!("{}", timer_id).into());
}

fn timer_info(ctx: &Context, args: Vec<RedisString>) -> RedisValueResult {
    let mut args = args.into_iter().skip(1);
    let timer_id = args.next_u64()?;

    let (remaining, data): (_, &MyData) = ctx.get_timer_info(timer_id)?;
    let reply = format!("Remaining: {:?}, data: {:?}", remaining, data);

    Ok(reply.into())
}

fn timer_stop(ctx: &Context, args: Vec<RedisString>) -> RedisValueResult {
    let mut args = args.into_iter().skip(1);
    let timer_id = args.next_u64()?;

    let data: MyData = ctx.stop_timer(timer_id)?;
    let reply = format!("Data: {:?}", data);

    Ok(reply.into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "timer",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["timer.create", timer_create, "", 0, 0, 0],
        ["timer.info", timer_info, "", 0, 0, 0],
        ["timer.stop", timer_stop, "", 0, 0, 0],
    ],
}
