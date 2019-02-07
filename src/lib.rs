#![allow(dead_code)]

use std::string;
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive_derive;
extern crate num_traits;

use libc::size_t;

pub mod alloc;
pub mod redisraw;
pub mod error;
pub mod raw;
pub mod native_types;

#[macro_use]
mod macros;
mod command;
mod context;
mod key;

pub use command::CommandOld;
pub use context::Context;

use crate::error::Error;

#[global_allocator]
static ALLOC: crate::alloc::RedisAlloc = crate::alloc::RedisAlloc;


/// `LogLevel` is a level of logging to be specified with a Redis log directive.
#[derive(Clone, Copy, Debug)]
pub enum LogLevel {
    Debug,
    Notice,
    Verbose,
    Warning,
}

/// Reply represents the various types of a replies that we can receive after
/// executing a Redis command.
#[derive(Debug)]
pub enum Reply {
    Array,
    Error,
    Integer(i64),
    Nil,
    String(String),
    Unknown,
}


/// `RedisString` is an abstraction over a Redis string.
///
/// Its primary function is to ensure the proper deallocation of resources when
/// it goes out of scope. Redis normally requires that strings be managed
/// manually by explicitly freeing them when you're done. This can be a risky
/// prospect, especially with mechanics like Rust's `?` operator, so we ensure
/// fault-free operation through the use of the Drop trait.
#[derive(Debug)]
pub struct RedisString {
    ctx: *mut raw::RedisModuleCtx,
    str_inner: *mut raw::RedisModuleString,
}

impl RedisString {
    fn create(ctx: *mut raw::RedisModuleCtx, s: &str) -> RedisString {
        let str = CString::new(s).unwrap();
        let str_inner = raw::create_string(ctx, str.as_ptr(), s.len());
        RedisString { ctx, str_inner }
    }
}

impl Drop for RedisString {
    // Frees resources appropriately as a RedisString goes out of scope.
    fn drop(&mut self) {
        raw::free_string(self.ctx, self.str_inner);
    }
}

fn manifest_redis_reply(
    reply: *mut raw::RedisModuleCallReply,
) -> Result<Reply, Error> {
    match raw::call_reply_type(reply) {
        raw::ReplyType::Integer => Ok(Reply::Integer(raw::call_reply_integer(reply))),
        raw::ReplyType::Nil => Ok(Reply::Nil),
        raw::ReplyType::String => {
            let mut length: size_t = 0;
            let bytes = raw::call_reply_string_ptr(reply, &mut length);
            from_byte_string(bytes, length)
                .map(Reply::String)
                .map_err(Error::from)
        }
        raw::ReplyType::Unknown => Ok(Reply::Unknown),

        // TODO: I need to actually extract the error from Redis here. Also, it
        // should probably be its own non-generic variety of Error.
        raw::ReplyType::Error => Err(error!("Redis replied with an error.")),

        other => Err(error!("Don't yet handle Redis type: {:?}", other)),
    }
}

fn manifest_redis_string(
    redis_str: *mut raw::RedisModuleString,
) -> Result<String, string::FromUtf8Error> {
    let mut length: size_t = 0;
    let bytes = raw::string_ptr_len(redis_str, &mut length);
    from_byte_string(bytes, length)
}

fn parse_args(
    argv: *mut *mut raw::RedisModuleString,
    argc: c_int,
) -> Result<Vec<String>, string::FromUtf8Error> {
    let mut args: Vec<String> = Vec::with_capacity(argc as usize);
    for i in 0..argc {
        let redis_str = unsafe { *argv.offset(i as isize) };
        args.push(manifest_redis_string(redis_str)?);
    }
    Ok(args)
}

fn from_byte_string(
    byte_str: *const c_char,
    length: size_t,
) -> Result<String, string::FromUtf8Error> {
    let mut vec_str: Vec<u8> = Vec::with_capacity(length as usize);
    for j in 0..length {
        let byte = unsafe { *byte_str.offset(j as isize) } as u8;
        vec_str.insert(j, byte);
    }

    String::from_utf8(vec_str)
}

