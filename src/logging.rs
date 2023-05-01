use crate::raw;
use std::ffi::CString;
use std::ptr;
use strum_macros::AsRefStr;

const NOT_INITIALISED_MESSAGE: &str = "Redis module hasn't been initialised.";

/// [RedisLogLevel] is a level of logging which can be used when
/// logging with Redis. See [raw::RedisModule_Log] and the official
/// redis [reference](https://redis.io/docs/reference/modules/modules-api-ref/).
#[derive(Clone, Copy, Debug, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum RedisLogLevel {
    Debug,
    Notice,
    Verbose,
    Warning,
}

pub(crate) fn log_internal(ctx: *mut raw::RedisModuleCtx, level: RedisLogLevel, message: &str) {
    if cfg!(test) {
        return;
    }

    let level = CString::new(level.as_ref()).unwrap();
    let fmt = CString::new(message).unwrap();
    unsafe {
        raw::RedisModule_Log.expect(NOT_INITIALISED_MESSAGE)(ctx, level.as_ptr(), fmt.as_ptr())
    }
}

/// This function should be used when a callback is returning a critical error
/// to the caller since cannot load or save the data for some critical reason.
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn log_io_error(io: *mut raw::RedisModuleIO, level: RedisLogLevel, message: &str) {
    if cfg!(test) {
        return;
    }
    let level = CString::new(level.as_ref()).unwrap();
    let fmt = CString::new(message).unwrap();
    unsafe {
        raw::RedisModule_LogIOError.expect(NOT_INITIALISED_MESSAGE)(
            io,
            level.as_ptr(),
            fmt.as_ptr(),
        )
    }
}

/// Log a message to the Redis log with the given log level, without
/// requiring a context. This prevents Redis from including the module
/// name in the logged message.
pub fn log<T: AsRef<str>>(level: RedisLogLevel, message: T) {
    log_internal(ptr::null_mut(), level, message.as_ref());
}

/// Log a message to Redis at the [RedisLogLevel::Debug] level.
pub fn log_debug<T: AsRef<str>>(message: T) {
    log(RedisLogLevel::Debug, message.as_ref());
}

/// Log a message to Redis at the [RedisLogLevel::Notice] level.
pub fn log_notice<T: AsRef<str>>(message: T) {
    log(RedisLogLevel::Notice, message.as_ref());
}

/// Log a message to Redis at the [RedisLogLevel::Verbose] level.
pub fn log_verbose<T: AsRef<str>>(message: T) {
    log(RedisLogLevel::Verbose, message.as_ref());
}

/// Log a message to Redis at the [RedisLogLevel::Warning] level.
pub fn log_warning<T: AsRef<str>>(message: T) {
    log(RedisLogLevel::Warning, message.as_ref());
}

/// The [log] crate implementation of logging.
pub mod standard_log_implementation {
    use super::*;
    use log::{Level, Metadata, Record, SetLoggerError};

    static LOGGER: RedisGlobalLogger = RedisGlobalLogger;

    /// The struct which has an implementation of the [log] crate's
    /// logging interface.
    ///
    /// # Note
    ///
    /// Redis does not support logging at the [log::Level::Error] level,
    /// so logging at this level will be converted to logging at the
    /// [log::Level::Warn] level under the hood.
    struct RedisGlobalLogger;

    /// Sets this logger as a global logger. Use this method to set
    /// up the logger. If this method is never called, the default
    /// logger is used which redirects the logging to the standard
    /// input/output streams.
    ///
    /// # Example
    ///
    /// This function may be called on a module startup, within the
    /// module initialisation function (specified in the
    /// [crate::redis_module] as the `init` argument, which will be used
    /// for the module initialisation and will be passed to the
    /// [raw::Export_RedisModule_Init] function when loading the
    /// module).
    #[allow(dead_code)]
    pub fn setup() -> Result<(), SetLoggerError> {
        log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Trace))
    }

    impl log::Log for RedisGlobalLogger {
        fn enabled(&self, _: &Metadata) -> bool {
            true
        }

        fn log(&self, record: &Record) {
            if !self.enabled(record.metadata()) {
                return;
            }

            let message = format!(
                "'{}' {}:{}: {}",
                record.module_path().unwrap_or_default(),
                record.file().unwrap_or("Unknown"),
                record.line().unwrap_or(0),
                record.args()
            );

            match record.level() {
                Level::Debug => log_debug(message),
                Level::Error | Level::Warn => log_warning(message),
                Level::Info => log_notice(message),
                Level::Trace => log_verbose(message),
            }
        }

        fn flush(&self) {
            // The flushing isn't required for the Redis logging.
        }
    }
}
pub use standard_log_implementation::*;
