#[macro_export]
macro_rules! redis_command {
    ($ctx:expr,
     $command_name:expr,
     $command_handler:expr,
     $command_flags:expr,
     $firstkey:expr,
     $lastkey:expr,
     $keystep:expr) => {{
        let name = CString::new($command_name).unwrap();
        let flags = CString::new($command_flags).unwrap();

        /////////////////////
        extern "C" fn do_command(
            ctx: *mut raw::RedisModuleCtx,
            argv: *mut *mut raw::RedisModuleString,
            argc: c_int,
        ) -> c_int {
            let context = Context::new(ctx);

            let args_decoded: Result<Vec<_>, RedisError> =
                unsafe { slice::from_raw_parts(argv, argc as usize) }
                    .into_iter()
                    .map(|&arg| {
                        RedisString::from_ptr(arg)
                            .map(|v| v.to_owned())
                            .map_err(|_| RedisError::Str("UTF8 encoding error in handler args"))
                    })
                    .collect();

            let response = args_decoded
                .map(|args| $command_handler(&context, args))
                .unwrap_or_else(|e| Err(e));

            context.reply(response) as c_int
        }
        /////////////////////

        if unsafe {
            raw::RedisModule_CreateCommand.unwrap()(
                $ctx,
                name.as_ptr(),
                Some(do_command),
                flags.as_ptr(),
                $firstkey,
                $lastkey,
                $keystep,
            )
        } == raw::Status::Err as c_int
        {
            return raw::Status::Err as c_int;
        }
    }};
}

#[macro_export]
macro_rules! redis_module {
    (
        name: $module_name:expr,
        version: $module_version:expr,
        data_types: [
            $($data_type:ident),* $(,)*
        ],
        $(init: $init_func:ident,)* $(,)*
        commands: [
            $([
                $name:expr,
                $command:expr,
                $flags:expr,
                $firstkey:expr,
                $lastkey:expr,
                $keystep:expr
              ]),* $(,)*
        ] $(,)*
    ) => {
        use std::os::raw::{c_int, c_char};
        use std::ffi::CString;
        use std::slice;

        use redis_module::raw;
        use redis_module::RedisString;

        #[no_mangle]
        #[allow(non_snake_case)]
        pub extern "C" fn RedisModule_OnLoad(
            ctx: *mut raw::RedisModuleCtx,
            _argv: *mut *mut raw::RedisModuleString,
            _argc: c_int,
        ) -> c_int {
            let mut name_buffer = [0; 64];
            let mut dest = name_buffer.as_mut_ptr();
            for byte in $module_name.chars() {
                unsafe {
                    *dest = byte as i8;
                    dest = dest.add(1);
                }
            }
            // We use an explicit block here to make sure all memory allocated before we
            // switch to the Redis allocator will be out of scope and thus deallocated.
            let module_version = $module_version as c_int;

            if unsafe { raw::Export_RedisModule_Init(
                ctx,
                name_buffer.as_ptr() as *const c_char,
                module_version,
                raw::REDISMODULE_APIVER_1 as c_int,
            ) } == raw::Status::Err as c_int { return raw::Status::Err as c_int; }

            $(
                if $init_func(ctx) == raw::Status::Err as c_int {
                    return raw::Status::Err as c_int;
                }
            )*

            $(
                if (&$data_type).create_data_type(ctx).is_err() {
                    return raw::Status::Err as c_int;
                }
            )*

            $(
                redis_command!(ctx, $name, $command, $flags, $firstkey, $lastkey, $keystep);
            )*

            raw::Status::Ok as c_int
        }
    }
}
