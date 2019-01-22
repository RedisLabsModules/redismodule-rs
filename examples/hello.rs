extern crate libc;

use libc::c_int;

extern crate redismodule;

use redismodule::error::Error;
use redismodule::Command;
use redismodule::raw;
use redismodule::Redis;

const MODULE_NAME: &str = "hello";
const MODULE_VERSION: c_int = 1;

struct HelloCommand;

impl Command for HelloCommand {
    fn name() -> &'static str { "hello.mul" }

    fn str_flags() -> &'static str { "write" }

    // Run the command.
    fn run(r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::generic(format!(
                "Usage: {} <m1> <m2>", Self::name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let m1 = parse_i64(args[1])?;
        let m2 = parse_i64(args[2])?;

        r.reply_array(3)?;
        r.reply_integer(m1)?;
        r.reply_integer(m2)?;
        r.reply_integer(m1 * m2)?;

        Ok(())
    }
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn HelloCommand_Redis(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {
    HelloCommand::execute(ctx, argv, argc).into()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut raw::RedisModuleCtx,
    _argv: *mut *mut raw::RedisModuleString,
    _argc: c_int,
) -> c_int {
    if raw::init(ctx, MODULE_NAME, MODULE_VERSION) == raw::Status::Err {
        return raw::Status::Err.into();
    }

    // TODO: Add this as a method on the Command trait?
    if raw::create_command(
        ctx,
        HelloCommand::name(),
        HelloCommand_Redis,
        HelloCommand::str_flags(),
        0,
        0,
        0,
    ) == raw::Status::Err {
        return raw::Status::Err.into();
    }

    raw::Status::Ok.into()
}

fn parse_i64(arg: &str) -> Result<i64, Error> {
    arg.parse::<i64>()
        .map_err(|_| Error::generic(format!("Couldn't parse as integer: {}", arg).as_str()))
}
