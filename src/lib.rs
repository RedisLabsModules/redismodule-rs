//#![allow(dead_code)]

use std::os::raw::c_char;
use std::str::Utf8Error;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate enum_primitive_derive;
extern crate num_traits;

use libc::size_t;

pub mod alloc;
pub mod error;
pub mod native_types;
pub mod raw;
pub mod rediserror;
mod redismodule;
pub mod redisraw;
pub mod redisvalue;

#[macro_use]
mod macros;
mod context;
mod key;

#[cfg(feature = "experimental-api")]
pub use crate::context::blocked::BlockedClient;
#[cfg(feature = "experimental-api")]
pub use crate::context::thread_safe::{DetachedFromClient, ThreadSafeContext};

pub use crate::context::Context;
pub use crate::redismodule::*;

/// Ideally this would be `#[cfg(not(test))]`, but that doesn't work:
/// https://github.com/rust-lang/rust/issues/59168#issuecomment-472653680
/// The workaround is to use the `test` feature instead.
#[cfg(not(feature = "test"))]
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

fn from_byte_string(byte_str: *const c_char, length: size_t) -> Result<String, Utf8Error> {
    let mut vec_str: Vec<u8> = Vec::with_capacity(length as usize);
    for j in 0..length {
        let byte = unsafe { *byte_str.add(j) } as u8;
        vec_str.insert(j, byte);
    }

    String::from_utf8(vec_str).map_err(|e| e.utf8_error())
}
