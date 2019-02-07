macro_rules! error {
    ($message:expr) => {
        Error::generic($message)
    };
    ($message:expr, $($arg:tt)*) => {
        Error::generic(format!($message, $($arg)+).as_str())
    }
}

#[allow(unused_macros)]
macro_rules! log_debug {
    ($logger:expr, $target:expr) => {
        if cfg!(debug_assertions) {
            $logger.log_debug($target)
        }
    };
    ($logger:expr, $target:expr, $($arg:tt)*) => {
        if cfg!(debug_assertions) {
            $logger.log_debug(format!($target, $($arg)+).as_str())
        }
    }
}

