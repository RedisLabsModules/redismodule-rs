//#![allow(dead_code)]

pub use crate::context::InfoContext;
use strum_macros::AsRefStr;
extern crate num_traits;

pub mod alloc;
pub mod error;
pub mod native_types;
pub mod raw;
pub mod rediserror;
mod redismodule;
pub mod redisraw;
pub mod redisvalue;
pub mod stream;

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

pub use crate::context::keys_cursor::KeysCursor;
pub use crate::context::Context;
pub use crate::context::ContextFlags;
pub use crate::context::AclPermissions;
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

pub fn base_info_func(
    ctx: &InfoContext,
    for_crash_report: bool,
    extended_info_func: Option<fn(&InfoContext, bool)>,
) {
    // If needed, add rust trace into the crash report (before module info)
    if for_crash_report && ctx.add_info_section(Some("trace")) == Status::Ok {
        let current_backtrace = Backtrace::new();
        let trace = format!("{current_backtrace:?}");
        ctx.add_info_field_str("trace", &trace);
    }

    if let Some(func) = extended_info_func {
        // Add module info
        func(ctx, for_crash_report);
    }
}
