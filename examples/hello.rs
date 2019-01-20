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
        "hello.world"
    }

    // Run the command.
    fn run(&self, r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::generic(format!(
                "Usage: {} <message> <times>",
                self.name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let message = args[1];
        let times = parse_i64(args[2])?;

        r.reply_array(2)?;
        r.reply_integer(42)?;
        r.reply_integer(43)?;

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
) -> raw::Status {
    Command::harness(&HelloCommand, ctx, argv, argc)
}

#[allow(non_snake_case)]
#[allow(unused_variables)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> raw::Status {
    if raw::init(
        ctx,
        MODULE_NAME,
        MODULE_VERSION,
        raw::REDISMODULE_APIVER_1,
    ) == raw::Status::Err
    {
        return raw::Status::Err;
    }

    let command = HelloCommand;
    // TODO: Add this as a method on the Command trait?
    if raw::create_command(
        ctx,
        command.name(),
        Some(Hello_RedisCommand),
        command.str_flags(),
        0,
        0,
        0,
    ) == raw::Status::Err
    {
        return raw::Status::Err;
    }

    raw::Status::Ok
}

fn parse_i64(arg: &str) -> Result<i64, Error> {
    arg.parse::<i64>()
        .map_err(|_| Error::generic(format!("Couldn't parse as integer: {}", arg).as_str()))
}
