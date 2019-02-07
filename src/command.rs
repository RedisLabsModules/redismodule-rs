use std::ffi::CString;
use std::os::raw::c_int;

use crate::raw;
use crate::error::Error;
use crate::context::Context;
use crate::parse_args;


/*
pub struct Command<F> where F: Fn(&Context, Vec<String>) -> RedisResult {
    pub name: &'static str,
    pub handler: F,
    pub flags: &'static str,
}
*/

type CommandFuncPtr = extern "C" fn(
    *mut raw::RedisModuleCtx,
    *mut *mut raw::RedisModuleString,
    c_int,
) -> c_int;


pub trait CommandOld {
    // Should return the name of the command to be registered.
    fn name() -> &'static str;

    fn external_command() -> CommandFuncPtr;

    // Should return any flags to be registered with the name as a string
    // separated list. See the Redis module API documentation for a complete
    // list of the ones that are available.
    fn str_flags() -> &'static str;

    // Run the command.
    fn run(r: Context, args: &[&str]) -> Result<(), Error>;

    /// Provides a basic wrapper for a command's implementation that parses
    /// arguments to Rust data types and handles the OK/ERR reply back to Redis.
    fn execute(
        ctx: *mut raw::RedisModuleCtx,
        argv: *mut *mut raw::RedisModuleString,
        argc: c_int,
    ) -> raw::Status {
        let args = parse_args(argv, argc).unwrap();
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let r = Context::new(ctx);

        match Self::run(r, str_args.as_slice()) {
            Ok(_) => raw::Status::Ok,
            Err(e) => {
                let message = format!("Redis error: {}", e.to_string());
                let message = CString::new(message).unwrap();

                raw::reply_with_error(
                    ctx,
                    message.as_ptr(),
                );

                raw::Status::Err
            }
        }
    }

    fn create(ctx: *mut raw::RedisModuleCtx) -> Result<(), &'static str> {
        raw::create_command(
            ctx,
            Self::name(),
            Self::external_command(),
            Self::str_flags(),
            0, 0, 0,
        )
    }
}

