//#![allow(dead_code)]

pub use crate::context::InfoContext;
use std::os::raw::c_char;
use std::str::Utf8Error;
use strum_macros::AsRefStr;
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

mod context;
pub mod key;
pub mod logging;
mod macros;
mod utils;

#[cfg(feature = "experimental-api")]
pub use crate::context::blocked::BlockedClient;
#[cfg(feature = "experimental-api")]
pub use crate::context::thread_safe::{DetachedFromClient, ThreadSafeContext};
#[cfg(feature = "experimental-api")]
pub use crate::raw::NotifyEvent;

pub use crate::context::Context;
pub use crate::raw::*;
pub use crate::redismodule::*;
use backtrace::Backtrace;

/// Ideally this would be `#[cfg(not(test))]`, but that doesn't work:
/// [59168#issuecomment-472653680](https://github.com/rust-lang/rust/issues/59168#issuecomment-472653680)
/// The workaround is to use the `test` feature instead.
#[cfg(not(feature = "test"))]
#[global_allocator]
static ALLOC: crate::alloc::RedisAlloc = crate::alloc::RedisAlloc;

/// `LogLevel` is a level of logging to be specified with a Redis log directive.
#[derive(Clone, Copy, Debug, AsRefStr)]
#[strum(serialize_all = "snake_case")]
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

pub fn base_info_func(
    ctx: &InfoContext,
    for_crash_report: bool,
    extended_info_func: Option<fn(&InfoContext, bool)>,
) {
    if !for_crash_report {
        if let Some(func) = extended_info_func {
            func(ctx, for_crash_report);
            return;
        }
    }
    // add rust trace into the crash report
    if ctx.add_info_section(Some("trace")) == Status::Ok {
        let current_backtrace = Backtrace::new();
        let trace = format!("{:?}", current_backtrace);
        ctx.add_info_field_str("trace", &trace);
    }
}
