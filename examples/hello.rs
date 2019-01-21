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
    // Should return the name of the command to be registered.
    fn name(&self) -> &'static str {
        "hello.mul"
    }

    // Run the command.
    fn run(&self, r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::generic(format!(
                "Usage: {} <m1> <m2>",
                self.name()
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

    // Should return any flags to be registered with the name as a string
    // separated list. See the Redis module API documentation for a complete
    // list of the ones that are available.
    fn str_flags(&self) -> &'static str {
        "write"
    }
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn Hello_RedisCommand(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {
    Command::harness(&HelloCommand, ctx, argv, argc).into()
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {

    if raw::init(ctx, MODULE_NAME, MODULE_VERSION) == raw::Status::Err {
        return raw::Status::Err.into();
    }

    let command = HelloCommand;
    // TODO: Add this as a method on the Command trait?
    if raw::create_command(
        ctx,
        command.name(),
        Hello_RedisCommand,
        command.str_flags(),
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
