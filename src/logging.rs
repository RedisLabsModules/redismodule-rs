use crate::LogLevel;
use crate::raw;
use std::ffi::CString;
use std::ptr;

pub(crate) fn log_internal(ctx: *mut raw::RedisModuleCtx, level: LogLevel, message: &str) {
    if cfg!(feature = "test") {
        return;
    }
    let level = CString::new(level.as_ref()).unwrap();
    let fmt = CString::new(message).unwrap();
    unsafe { raw::RedisModule_Log.unwrap()(ctx, level.as_ptr(), fmt.as_ptr()) }
}

/// Log a message to the Redis log with the given log level, without
/// requiring a context. This prevents Redis from including the module
/// name in the logged message.
pub fn log(level: LogLevel, message: &str) {
    log_internal(ptr::null_mut(), level, message);
}

/// Log a message to the Redis log with DEBUG log level.
pub fn log_debug(message: &str) {
    log(LogLevel::Debug, message);
}

/// Log a message to the Redis log with NOTICE log level.
pub fn log_notice(message: &str) {
    log(LogLevel::Notice, message);
}

/// Log a message to the Redis log with VERBOSE log level.
pub fn log_verbose(message: &str) {
    log(LogLevel::Verbose, message);
}

/// Log a message to the Redis log with WARNING log level.
pub fn log_warning(message: &str) {
    log(LogLevel::Warning, message);
}
