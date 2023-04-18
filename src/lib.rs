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

pub mod configuration;
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

pub use crate::configuration::ConfigurationValue;
pub use crate::configuration::EnumConfigurationValue;
pub use crate::context::call_reply::{CallReply, CallResult, ErrorReply};
pub use crate::context::keys_cursor::KeysCursor;
pub use crate::context::server_events;
pub use crate::context::thread_safe::ContextGuard;
pub use crate::context::thread_safe::RedisGILGuard;
pub use crate::context::thread_safe::RedisLockIndicator;
pub use crate::context::AclPermissions;
pub use crate::context::CallOptionResp;
pub use crate::context::CallOptions;
pub use crate::context::CallOptionsBuilder;
pub use crate::context::Context;
pub use crate::context::ContextFlags;
pub use crate::context::DetachedContext;
pub use crate::raw::*;
pub use crate::redismodule::*;
use backtrace::Backtrace;

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

/// Initialize RedisModuleAPI without register as a module.
pub fn init_api(ctx: &Context) {
    unsafe { crate::raw::Export_RedisModule_InitAPI(ctx.ctx) };
}
