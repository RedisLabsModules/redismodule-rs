pub use crate::context::InfoContext;
extern crate num_traits;

pub mod alloc;
pub mod apierror;
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

pub use crate::context::blocked::BlockedClient;
pub use crate::context::thread_safe::{
    ContextGuard, DetachedFromClient, RedisGILGuard, RedisLockIndicator, ThreadSafeContext,
};
pub use crate::raw::NotifyEvent;

pub use crate::configuration::ConfigurationValue;
pub use crate::configuration::EnumConfigurationValue;
pub use crate::context::call_reply::FutureCallReply;
pub use crate::context::call_reply::{CallReply, CallResult, ErrorReply, PromiseCallReply};
pub use crate::context::commands;
pub use crate::context::defrag;
pub use crate::context::keys_cursor::KeysCursor;
pub use crate::context::key_scan_cursor::ScanKeyCursor;
pub use crate::context::server_events;
pub use crate::context::AclCategory;
pub use crate::context::AclPermissions;
#[cfg(any(
    feature = "min-redis-compatibility-version-7-4",
    feature = "min-redis-compatibility-version-7-2"
))]
pub use crate::context::BlockingCallOptions;
pub use crate::context::CallOptionResp;
pub use crate::context::CallOptions;
pub use crate::context::CallOptionsBuilder;
pub use crate::context::Context;
pub use crate::context::ContextFlags;
pub use crate::context::DetachedContext;
pub use crate::context::DetachedContextGuard;
pub use crate::context::{
    InfoContextBuilderFieldBottomLevelValue, InfoContextBuilderFieldTopLevelValue,
    InfoContextFieldBottomLevelData, InfoContextFieldTopLevelData, OneInfoSectionData,
};
pub use crate::raw::*;
pub use crate::redismodule::*;
use backtrace::Backtrace;
use context::server_events::INFO_COMMAND_HANDLER_LIST;

/// The detached Redis module context (the context of this module). It
/// is only set to a proper value after the module is initialised via the
/// provided [redis_module] macro.
/// See [DetachedContext].
pub static MODULE_CONTEXT: DetachedContext = DetachedContext::new();

#[deprecated(
    since = "2.1.0",
    note = "Please use the redis_module::logging::RedisLogLevel directly instead."
)]
pub type LogLevel = logging::RedisLogLevel;

fn add_trace_info(ctx: &InfoContext) -> RedisResult<()> {
    const SECTION_NAME: &str = "trace";
    const FIELD_NAME: &str = "backtrace";

    let current_backtrace = Backtrace::new();
    let trace = format!("{current_backtrace:?}");

    ctx.builder()
        .add_section(SECTION_NAME)
        .field(FIELD_NAME, trace)?
        .build_section()?
        .build_info()?;

    Ok(())
}

/// A type alias for the custom info command handler.
/// The function may optionally return an object of one section to add.
/// If nothing is returned, it is assumed that the function has already
/// filled all the information required via [`InfoContext::builder`].
pub type InfoHandlerFunctionType = fn(&InfoContext, bool) -> RedisResult<()>;

/// Default "INFO" command handler for the module.
///
/// This function can be invoked, for example, by sending `INFO modules`
/// through the RESP protocol.
pub fn basic_info_command_handler(ctx: &InfoContext, for_crash_report: bool) {
    if for_crash_report {
        if let Err(e) = add_trace_info(ctx) {
            log::error!("Couldn't send info for the module: {e}");
            return;
        }
    }

    INFO_COMMAND_HANDLER_LIST
        .iter()
        .filter_map(|callback| callback(ctx, for_crash_report).err())
        .for_each(|e| log::error!("Couldn't build info for the module's custom handler: {e}"));
}

/// Initialize RedisModuleAPI without register as a module.
pub fn init_api(ctx: &Context) {
    unsafe { crate::raw::Export_RedisModule_InitAPI(ctx.ctx) };
}

pub(crate) unsafe fn deallocate_pointer<P>(p: *mut P) {
    std::ptr::drop_in_place(p);
    std::alloc::dealloc(p as *mut u8, std::alloc::Layout::new::<P>());
}
