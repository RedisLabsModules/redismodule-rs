use std::os::raw::c_void;

extern crate libc;

use libc::c_int;

extern crate redismodule;

use redismodule::error::Error;
use redismodule::Command;
use redismodule::raw;
use redismodule::Redis;
use redismodule::raw::module_init;
use redismodule::types::RedisType;

const MODULE_NAME: &str = "alloc";
const MODULE_VERSION: c_int = 1;

#[allow(unused)]
struct MyType {
    data: String,
}

static MY_TYPE: RedisType = RedisType::new();

struct AllocSetCommand;

impl Command for AllocSetCommand {
    fn name() -> &'static str { "alloc.set" }

    fn external_command() -> raw::CommandFunc { AllocSetCommand_Redis }

    fn str_flags() -> &'static str { "write" }

    // Run the command.
    fn run(r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 3 {
            // FIXME: Use RedisModule_WrongArity instead. Return an ArityError here and
            // in the low-level implementation call RM_WrongArity.
            return Err(Error::generic(format!(
                "Usage: {} <key> <size>", Self::name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let key = args[1];
        let size = parse_i64(args[2])?;

        // TODO:
        // 1. Open key [OK]
        // 2. Allocate data [OK]
        // 3. Set the key to the data [OK]
        // 4. Activate custom allocator and compare Redis memory usage [OK]
        // 5. Handle deallocation of existing value [OK]

        let key = r.open_key_writable(key);
        let key_type = key.verify_and_get_type(&MY_TYPE)?;

        let my = match key_type {
            raw::KeyType::Empty => {
                // Create a new value
                Box::new(
                    MyType {
                        data: "A".repeat(size as usize)
                    }
                )
            }
            _ => {
                // There is an existing value, reuse it
                let my = key.get_value() as *mut MyType;

                if my.is_null() {
                    r.reply_integer(0)?;
                    return Ok(());
                }

                let mut my = unsafe { Box::from_raw(my) };
                my.data = "B".repeat(size as usize);
                my
            }
        };

        let my = Box::into_raw(my);

        key.set_value(&MY_TYPE, my as *mut c_void)?;
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

struct AllocGetCommand;

impl Command for AllocGetCommand {
    fn name() -> &'static str { "alloc.get" }

    fn external_command() -> raw::CommandFunc { AllocGetCommand_Redis }

    fn str_flags() -> &'static str { "" }

    // Run the command.
    fn run(r: Redis, args: &[&str]) -> Result<(), Error> {
        if args.len() != 2 {
            // FIXME: Use RedisModule_WrongArity instead. Return an ArityError here and
            // in the low-level implementation call RM_WrongArity.
            return Err(Error::generic(format!(
                "Usage: {} <key>", Self::name()
            ).as_str()));
        }

        // the first argument is command name (ignore it)
        let key = args[1];

        let key = r.open_key(key);
        key.verify_and_get_type(&MY_TYPE)?;
        let my = key.get_value() as *mut MyType;

        if my.is_null() {
            r.reply_integer(0)?;
            return Ok(());
        }

        let my = unsafe { &mut *my };
        let size = my.data.len();

        r.reply_array(2)?;
        r.reply_integer(size as i64)?;
        r.reply_string(my.data.as_str())?;

        Ok(())
    }
}

#[allow(non_snake_case)]
pub extern "C" fn AllocGetCommand_Redis(
    ctx: *mut raw::RedisModuleCtx,
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> c_int {
    AllocGetCommand::execute(ctx, argv, argc).into()
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
        let _key = args[1];

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

// TODO: Call this from inside module_init
    redismodule::use_redis_alloc();

    MY_TYPE.create_data_type(ctx, "mytype123")?;

    AllocSetCommand::create(ctx)?;
    AllocGetCommand::create(ctx)?;
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
