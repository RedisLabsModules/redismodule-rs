//#![allow(dead_code)]

use std::string;
use std::os::raw::c_char;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive_derive;
extern crate num_traits;

use libc::size_t;

pub mod redismodule;
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

pub use command::Command;
pub use context::Context;
pub use redismodule::*;

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

