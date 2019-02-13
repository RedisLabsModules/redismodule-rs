#[macro_export]
macro_rules! redis_command {
    ($ctx: expr, $command_name:expr, $command_handler:expr, $command_flags:expr) => {
        {
            let name = CString::new($command_name).unwrap();
            let flags = CString::new($command_flags).unwrap();
            let (firstkey, lastkey, keystep) = (1, 1, 1);

            /////////////////////
            extern fn do_command(
                ctx: *mut raw::RedisModuleCtx,
                argv: *mut *mut raw::RedisModuleString,
                argc: c_int,
            ) -> c_int {
                let context = Context::new(ctx);

                let args: Vec<String> = unsafe { slice::from_raw_parts(argv, argc as usize) }
                    .into_iter()
                    .map(|a| RedisString::from_ptr(*a).expect("UTF8 encoding error in handler args").to_string())
                    .collect();

                let response = $command_handler(&context, args);
                context.reply(response) as c_int
            }
            /////////////////////

            if raw::RedisModule_CreateCommand.unwrap()(
                $ctx,
                name.as_ptr(),
                Some(do_command),
                flags.as_ptr(),
                firstkey, lastkey, keystep,
            ) == raw::Status::Err as c_int { return raw::Status::Err as c_int; }
        }
    }
}

#[macro_export]
macro_rules! redis_module {
    (
        name: $module_name:expr,
        version: $module_version:expr,
        data_types: [
            $($data_type:ident),* $(,)*
        ],
        commands: [
            $([
                $name:expr,
                $command:ident,
                $flags:expr
              ]),* $(,)*
        ] $(,)*
    ) => {
        use std::os::raw::c_int;
        use std::ffi::CString;
        use std::slice;

        use redismodule::raw;
        use redismodule::RedisString;

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
                ) == raw::Status::Err as c_int { return raw::Status::Err as c_int; }

                $(
                    if (&$data_type).create_data_type(ctx).is_err() {
                        return raw::Status::Err as c_int;
                    }
                )*

                if true {
                    redismodule::alloc::use_redis_alloc();
                } else {
                    eprintln!("*** NOT USING Redis allocator ***");
                }

                $(
                    redis_command!(ctx, $name, $command, $flags);
                )*

                raw::Status::Ok as c_int
            }
        }
    }
}
