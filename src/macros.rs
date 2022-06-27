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
        extern "C" fn __do_command(
            ctx: *mut $crate::raw::RedisModuleCtx,
            argv: *mut *mut $crate::raw::RedisModuleString,
            argc: c_int,
        ) -> c_int {
            let context = $crate::Context::new(ctx);

            let args = $crate::decode_args(ctx, argv, argc);
            let response = $command_handler(&context, args);
            context.reply(response) as c_int
        }
        /////////////////////

        if unsafe {
            $crate::raw::RedisModule_CreateCommand.unwrap()(
                $ctx,
                name.as_ptr(),
                Some(__do_command),
                flags.as_ptr(),
                $firstkey,
                $lastkey,
                $keystep,
            )
        } == $crate::raw::Status::Err as c_int
        {
            return $crate::raw::Status::Err as c_int;
        }
    }};
}

#[cfg(feature = "experimental-api")]
#[macro_export]
macro_rules! redis_event_handler {
    (
        $ctx: expr,
        $event_type: expr,
        $event_handler: expr
    ) => {{
        extern "C" fn __handle_event(
            ctx: *mut $crate::raw::RedisModuleCtx,
            event_type: c_int,
            event: *const c_char,
            key: *mut $crate::raw::RedisModuleString,
        ) -> c_int {
            let context = $crate::Context::new(ctx);

            let redis_key = $crate::RedisString::from_ptr(key).unwrap();
            let event_str = unsafe { CStr::from_ptr(event) };
            $event_handler(
                &context,
                $crate::NotifyEvent::from_bits_truncate(event_type),
                event_str.to_str().unwrap(),
                redis_key,
            );

            $crate::raw::Status::Ok as c_int
        }

        if unsafe {
            $crate::raw::RedisModule_SubscribeToKeyspaceEvents.unwrap()(
                $ctx,
                $event_type.bits(),
                Some(__handle_event),
            )
        } == $crate::raw::Status::Err as c_int
        {
            return $crate::raw::Status::Err as c_int;
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
        $(post_init: $post_init_func:ident,)* $(,)*
        $(deinit: $deinit_func:ident,)* $(,)*
        $(info: $info_func:ident,)?
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
        $(event_handlers: [
            $([
                $(@$event_type:ident) +:
                $event_handler:expr
            ]),* $(,)*
        ])?
        $(, server_events: [
            $([
                @$server_event_type:ident:
                $server_event_handler:expr
            ]),* $(,)*
        ])?
        $(, string_configurations: [
            $($string_config_ctx:expr),* $(,)*
        ])?
        $(, bool_configurations: [
            $($bool_config_ctx:expr),* $(,)*
        ])?
        $(, numeric_configurations: [
            $($numeric_config_ctx:expr),* $(,)*
        ])?
    ) => {
        extern "C" fn __info_func(
            ctx: *mut $crate::raw::RedisModuleInfoCtx,
            for_crash_report: i32,
        ) {
            use $crate::InfoContext;
            let mut __info_func__cb : Option<fn(&InfoContext, bool)> = None;
            $( __info_func__cb = Some($info_func); )?
            $crate::base_info_func(&$crate::InfoContext::new(ctx), for_crash_report == 1, __info_func__cb);
        }

        #[no_mangle]
        #[allow(non_snake_case)]
        pub unsafe extern "C" fn RedisModule_OnLoad(
            ctx: *mut $crate::raw::RedisModuleCtx,
            argv: *mut *mut $crate::raw::RedisModuleString,
            argc: std::os::raw::c_int,
        ) -> std::os::raw::c_int {
            use std::os::raw::{c_int, c_char};
            use std::ffi::{CString, CStr};

            use $crate::raw;
            use $crate::RedisString;
            use $crate::context::configuration;

            // We use a statically sized buffer to avoid allocating.
            // This is needed since we use a custom allocator that relies on the Redis allocator,
            // which isn't yet ready at this point.
            let mut name_buffer = [0; 64];
            unsafe {
                std::ptr::copy(
                    $module_name.as_ptr(),
                    name_buffer.as_mut_ptr(),
                    $module_name.len(),
                );
            }

            let module_version = $module_version as c_int;

            if unsafe { raw::Export_RedisModule_Init(
                ctx,
                name_buffer.as_ptr() as *const c_char,
                module_version,
                raw::REDISMODULE_APIVER_1 as c_int,
            ) } == raw::Status::Err as c_int { return raw::Status::Err as c_int; }

            let context = $crate::Context::new(ctx);
            let args = $crate::decode_args(ctx, argv, argc);

            $(
                if $init_func(&context, &args) == $crate::Status::Err {
                    return $crate::Status::Err as c_int;
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

            $(
                $(
                    redis_event_handler!(ctx, $(raw::NotifyEvent::$event_type |)+ raw::NotifyEvent::empty(), $event_handler);
                )*
            )?

            $(
                $(
                    if let Err(err) = (crate::redis_module::context::server_events::subscribe_to_server_event(
                        &context,
                        crate::redis_module::context::server_events::ServerEvents::$server_event_type,
                        Box::new($server_event_handler))) {
                            context.log_warning(&format!("Failed register server event, {}", err));
                            return $crate::Status::Err as c_int;
                        }
                )*
            )?

            if let Some(load_config) = raw::RedisModule_LoadConfigs.as_ref() {
                // configuration api only available on Redis 7.0, if its not available
                // ignore it. User will have to find another way to read configuration.
                $(
                    $(
                        if let Err(err) = configuration::register_string_configuration(&context, $string_config_ctx) {
                            context.log_warning(&format!("Failed register string configuration, {}", err));
                            return $crate::Status::Err as c_int;
                        }
                    )*
                )?

                $(
                    $(
                        if let Err(err) = configuration::register_bool_configuration(&context, $bool_config_ctx) {
                            context.log_warning(&format!("Failed register string configuration, {}", err));
                            return $crate::Status::Err as c_int;
                        }
                    )*
                )?

                $(
                    $(
                        if let Err(err) = configuration::register_numeric_configuration(&context, $numeric_config_ctx) {
                            context.log_warning(&format!("Failed register string configuration, {}", err));
                            return $crate::Status::Err as c_int;
                        }
                    )*
                )?

                load_config(ctx);
            }

            raw::register_info_function(ctx, Some(__info_func));

            $(
                if $post_init_func(&context) == $crate::Status::Err {
                    return $crate::Status::Err as c_int;
                }
            )*

            raw::Status::Ok as c_int
        }

        #[no_mangle]
        #[allow(non_snake_case)]
        pub extern "C" fn RedisModule_OnUnload(
            ctx: *mut $crate::raw::RedisModuleCtx
        ) -> std::os::raw::c_int {
            use std::os::raw::c_int;

            let context = $crate::Context::new(ctx);
            $(
                if $deinit_func(&context) == $crate::Status::Err {
                    return $crate::Status::Err as c_int;
                }
            )*

            $crate::raw::Status::Ok as c_int
        }
    }
}
