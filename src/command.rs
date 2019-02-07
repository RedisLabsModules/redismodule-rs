use std::slice;
use std::mem;

use crate::raw;
use crate::context::Context;
use crate::{RedisString, RedisResult};

pub struct Command<F> where F: Fn(&Context, Vec<String>) -> RedisResult {
    pub name: &'static str,
    pub flags: &'static str,
    pub handler: F,
}

impl<F> Command<F>
    where F: Fn(&Context, Vec<String>) -> RedisResult {
    pub fn new(name: &'static str, handler: F, flags: &'static str) -> Command<F> {
        Command {
            name,
            handler,
            flags,
        }
    }

    pub fn wrap_handler(&self) -> raw::RedisModuleCmdFunc {
        extern "C" fn do_command<F: Fn(&Context, Vec<String>) -> RedisResult>(
            ctx: *mut raw::RedisModuleCtx,
            argv: *mut *mut raw::RedisModuleString,
            argc: libc::c_int,
        ) -> i32 {
            unsafe {
                let cmd: *const F = &() as *const () as *const F;
                //let cmd: *const F = mem::transmute(&()); // equiv  ^^

                let context = Context::new(ctx);

                let args: Vec<String> = slice::from_raw_parts(argv, argc as usize)
                    .into_iter()
                    .map(|a| RedisString::from_ptr(*a).expect("UTF8 encoding error in handler args").to_string())
                    .collect();

                context.reply((*cmd)(&context, args)) as _
            }
        }

        assert_eq!(mem::size_of::<F>(), 0);

        Some(do_command::<F> as _)
    }
}
