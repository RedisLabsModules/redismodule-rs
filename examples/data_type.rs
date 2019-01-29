extern crate libc;

use libc::c_int;

extern crate redismodule;

use redismodule::error::Error;
use redismodule::Command;
use redismodule::raw;
use redismodule::Redis;
use redismodule::raw::module_init;
use redismodule::types::RedisModuleType;

const MODULE_NAME: &str = "alloc";
const MODULE_VERSION: c_int = 1;

// TODO: Can we use a safe smart pointer instead of an unsafe mutable static variable?
static mut MY_TYPE: RedisModuleType = RedisModuleType::new();

struct AllocSetCommand;

impl Command for AllocSetCommand {
    fn name() -> &'static str { "alloc.set" }

    fn external_command() -> raw::CommandFunc { AllocSetCommand_Redis }

    fn str_flags() -> &'static str { "write" }

    // Run the command.
    fn run(r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 3 {
            return Err(Error::generic(format!(
                "Usage: {} <key> <size>", Self::name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let key = args[1];
        let size = parse_i64(args[2])?;

        // TODO:
        // 1. Open key
        // 2. Allocate data
        // 3. Set the key to the data
        // 4. Activate custom allocator and compare Redis memory usage
        let data: Vec<u8> = Vec::with_capacity(size as usize);
        let k = r.open_key_writable(key);

        /*
        raw::RedisModule_ModuleTypeSetValue.unwrap()(
            k,
            t,
            data,
        );
        */

        r.reply_integer(size)?;

        Ok(())
    }
}

#[allow(non_snake_case)]
pub extern "C" fn AllocSetCommand_Redis(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {
    AllocSetCommand::execute(ctx, argv, argc).into()
}

//////////////////////////////////////////////////////

struct AllocDelCommand;

impl Command for AllocDelCommand {
    fn name() -> &'static str { "alloc.del" }

    fn external_command() -> raw::CommandFunc { AllocDelCommand_Redis }

    fn str_flags() -> &'static str { "write" }

    // Run the command.
    fn run(r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 2 {
            // FIXME: Use RedisModule_WrongArity instead?
            return Err(Error::generic(format!(
                "Usage: {} <key>", Self::name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let key = args[1];

        r.reply_string("OK")?;

        Ok(())
    }
}

// TODO: Write a macro to generate these glue functions
// TODO: Look at https://github.com/faineance/redismodule which has some macros

#[allow(non_snake_case)]
pub extern "C" fn AllocDelCommand_Redis(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {
    AllocDelCommand::execute(ctx, argv, argc).into()
}

fn module_on_load(ctx: *mut raw::RedisModuleCtx) -> Result<(), &'static str> {
    module_init(ctx, MODULE_NAME, MODULE_VERSION)?;

    // FIXME: Make this safe (use a smart pointer?)
    unsafe { MY_TYPE.create_data_type(ctx, "mytype123") }?;

    AllocSetCommand::create(ctx)?;
    AllocDelCommand::create(ctx)?;

    Ok(())
}

#[allow(non_snake_case)]
#[no_mangle]
pub extern "C" fn RedisModule_OnLoad(
    ctx: *mut raw::RedisModuleCtx,
    _argv: *mut *mut raw::RedisModuleString,
    _argc: c_int,
) -> c_int {
    if let Err(msg) = module_on_load(ctx) {
        eprintln!("Error loading module: {}", msg);
        return raw::Status::Err.into();
    }

    raw::Status::Ok.into()
}

fn parse_i64(arg: &str) -> Result<i64, Error> {
    arg.parse::<i64>()
        .map_err(|_| Error::generic(format!("Couldn't parse as integer: {}", arg).as_str()))
}
