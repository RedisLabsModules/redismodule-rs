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

#[macro_export]
macro_rules! redis_module (
    ($module_name:expr, $module_version:expr, $data_types:expr, $commands:expr) => (
        use std::os::raw::c_int;
        use std::ffi::CString;

        use redismodule::raw;

        #[no_mangle]
        #[allow(non_snake_case)]
        pub extern "C" fn RedisModule_OnLoad(
            ctx: *mut raw::RedisModuleCtx,
            _argv: *mut *mut raw::RedisModuleString,
            _argc: c_int,
        ) -> c_int {
            unsafe {
                let module_name = CString::new($module_name).unwrap();
                let module_version = $module_version as c_int;

                if raw::Export_RedisModule_Init(
                    ctx,
                    module_name.as_ptr(),
                    module_version,
                    raw::REDISMODULE_APIVER_1 as c_int,
                ) == raw::Status::Err as _ { return raw::Status::Err as _; }

                for data_type in &$data_types {
                    if data_type.create_data_type(ctx).is_err() {
                        return raw::Status::Err as _;
                    }
                }

                if true {
                    redismodule::alloc::use_redis_alloc();
                } else {
                    eprintln!("*** NOT USING Redis allocator ***");
                }

                for command in &$commands {
                    let name = CString::new(command.name).unwrap();
                    let flags = CString::new(command.flags).unwrap();
                    let (firstkey, lastkey, keystep) = (1, 1, 1);

                    if raw::RedisModule_CreateCommand.unwrap()(
                        ctx,
                        name.as_ptr(),
                        command.wrap_handler(),
                        flags.as_ptr(),
                        firstkey, lastkey, keystep,
                    ) == raw::Status::Err as _ { return raw::Status::Err as _; }
                }

                raw::Status::Ok as _
            }
        }
    )
);
