#[macro_export]
macro_rules! redis_command {
    ($ctx:expr,
     $command_name:expr,
     $command_handler:expr,
     $command_flags:expr,
     $firstkey:expr,
     $lastkey:expr,
     $keystep:expr,
     $mandatory_acl_categories:expr
     $(, $optional_acl_categories:expr)?
    ) => {{
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
            context.reply(response.map(|v| v.into())) as c_int
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
            $crate::raw::redis_log(
                $ctx,
                &format!("Error: failed to create command {}", $command_name),
            );
            return $crate::raw::Status::Err as c_int;
        }

        let command =
            unsafe { $crate::raw::RedisModule_GetCommand.unwrap()($ctx, name.as_ptr()) };
        if command.is_null() {
            $crate::raw::redis_log(
                $ctx,
                &format!("Error: failed to get command {}", $command_name),
            );
            return $crate::raw::Status::Err as c_int;
        }

        if let Some(RM_SetCommandACLCategories) = $crate::raw::RedisModule_SetCommandACLCategories {
            let mut acl_categories = CString::new("").unwrap();
            $(
                if $mandatory_acl_categories != "" && $optional_acl_categories != "" {
                    acl_categories = CString::new(format!("{} {}", $mandatory_acl_categories, $optional_acl_categories)).unwrap();
                } else if $optional_acl_categories != "" {
                    acl_categories = CString::new($optional_acl_categories).unwrap();
                }
                // Warn if optional ACL categories are not set, but don't fail.
                if RM_SetCommandACLCategories(command, acl_categories.as_ptr()) == $crate::raw::Status::Err as c_int {
                    $crate::raw::redis_log(
                        $ctx,
                        &format!(
                            "Warning: failed to set command `{}` ACL categories `{}`",
                            $command_name, acl_categories.to_str().unwrap()
                        ),
                    );
                }
            )?
            if $mandatory_acl_categories != "" {
                acl_categories = CString::new($mandatory_acl_categories).unwrap();

                // Fail if mandatory ACL categories are not set.
                if RM_SetCommandACLCategories(command, acl_categories.as_ptr())
                    == $crate::raw::Status::Err as c_int
                {
                    $crate::raw::redis_log(
                        $ctx,
                        &format!(
                            "Error: failed to set command `{}` mandatory ACL categories `{}`",
                            $command_name, $mandatory_acl_categories
                        ),
                    );
                    return $crate::raw::Status::Err as c_int;
                }
            }
        } else if $mandatory_acl_categories != "" {
            $crate::raw::redis_log(
                $ctx,
                "Warning: Redis version does not support ACL categories",
            );
            return $crate::raw::Status::Err as c_int;
        }
    }};
}

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

            let redis_key = $crate::RedisString::string_as_slice(key);
            let event_str = unsafe { CStr::from_ptr(event) };
            $event_handler(
                &context,
                $crate::NotifyEvent::from_bits_truncate(event_type),
                event_str.to_str().unwrap(),
                redis_key,
            );

            $crate::raw::Status::Ok as c_int
        }

        let all_available_notification_flags = $crate::raw::get_keyspace_notification_flags_all();
        let available_wanted_notification_flags = $event_type.intersection(all_available_notification_flags);
        if !all_available_notification_flags.contains($event_type) {
            let not_supported = $event_type.difference(all_available_notification_flags);
            $crate::Context::new($ctx).log_notice(&format!(
                "These event notification flags set aren't supported: {not_supported:?}. These flags will be used: {available_wanted_notification_flags:?}"
            ));
        }

        if !available_wanted_notification_flags.is_empty() && unsafe {
            $crate::raw::RedisModule_SubscribeToKeyspaceEvents.unwrap()(
                $ctx,
                available_wanted_notification_flags.bits(),
                Some(__handle_event),
            )
        } == $crate::raw::Status::Err as c_int
        {
            return $crate::raw::Status::Err as c_int;
        }
    }};
}

/// Defines a Redis module.
///
/// It registers the defined module, sets it up and initialises properly,
/// registers all the commands and types.
#[macro_export]
macro_rules! redis_module {
    (
        name: $module_name:expr,
        version: $module_version:expr,
        /// Global allocator for the redis module defined.
        /// In most of the cases, the Redis allocator ([crate::alloc::RedisAlloc])
        /// should be used.
        allocator: ($allocator_type:ty, $allocator_init:expr),
        data_types: [
            $($data_type:ident),* $(,)*
        ],
        // eg: `acl_category: "name_of_module_acl_category",`
        // This will add the specified (optional) ACL categories.
        $(acl_category: $module_acl_categories:expr,)* $(,)*
        $(init: $init_func:ident,)* $(,)*
        $(deinit: $deinit_func:ident,)* $(,)*
        $(info: $info_func:ident,)?
        commands: [
            $([
                $name:expr,
                $command:expr,
                $flags:expr,
                $firstkey:expr,
                $lastkey:expr,
                $keystep:expr,
                $mandatory_command_acl_categories:expr
                $(, $optional_command_acl_categories:expr)?
              ]),* $(,)*
        ] $(,)*
        $(event_handlers: [
            $([
                $(@$event_type:ident) +:
                $event_handler:expr
            ]),* $(,)*
        ] $(,)* )?
        $(configurations: [
            $(i64:[$([
                $i64_configuration_name:expr,
                $i64_configuration_val:expr,
                $i64_default:expr,
                $i64_min:expr,
                $i64_max:expr,
                $i64_flags_options:expr,
                $i64_on_changed:expr
            ]),* $(,)*],)?
            $(string:[$([
                $string_configuration_name:expr,
                $string_configuration_val:expr,
                $string_default:expr,
                $string_flags_options:expr,
                $string_on_changed:expr
            ]),* $(,)*],)?
            $(bool:[$([
                $bool_configuration_name:expr,
                $bool_configuration_val:expr,
                $bool_default:expr,
                $bool_flags_options:expr,
                $bool_on_changed:expr
            ]),* $(,)*],)?
            $(enum:[$([
                $enum_configuration_name:expr,
                $enum_configuration_val:expr,
                $enum_default:expr,
                $enum_flags_options:expr,
                $enum_on_changed:expr
            ]),* $(,)*],)?
            $(module_args_as_configuration:$use_module_args:expr,)?
            $(module_config_get:$module_config_get_command:expr,)?
            $(module_config_set:$module_config_set_command:expr,)?
        ])?
    ) => {
        /// Redis module allocator.
        #[global_allocator]
        static REDIS_MODULE_ALLOCATOR: $allocator_type = $allocator_init;

        // The old-style info command handler, if specified.
        $(
            #[redis_module_macros::info_command_handler]
            #[inline]
            fn module_info(ctx: &InfoContext, for_crash_report: bool) -> RedisResult<()> {
                $info_func(ctx, for_crash_report);

                Ok(())
            }
        )?

        extern "C" fn __info_func(
            ctx: *mut $crate::raw::RedisModuleInfoCtx,
            for_crash_report: i32,
        ) {
            $crate::basic_info_command_handler(&$crate::InfoContext::new(ctx), for_crash_report == 1);
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
            use $crate::server_events::register_server_events;
            use $crate::configuration::register_i64_configuration;
            use $crate::configuration::register_string_configuration;
            use $crate::configuration::register_bool_configuration;
            use $crate::configuration::register_enum_configuration;
            use $crate::configuration::module_config_get;
            use $crate::configuration::module_config_set;
            use $crate::configuration::get_i64_default_config_value;
            use $crate::configuration::get_string_default_config_value;
            use $crate::configuration::get_bool_default_config_value;
            use $crate::configuration::get_enum_default_config_value;

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
                name_buffer.as_ptr().cast::<c_char>(),
                module_version,
                raw::REDISMODULE_APIVER_1 as c_int,
            ) } == raw::Status::Err as c_int { return raw::Status::Err as c_int; }

            let context = $crate::Context::new(ctx);
            unsafe {
                let _ = $crate::MODULE_CONTEXT.set_context(&context);
            }
            let args = $crate::decode_args(ctx, argv, argc);

            $(
                if (&$data_type).create_data_type(ctx).is_err() {
                    return raw::Status::Err as c_int;
                }
            )*

            $(
                let categories = CString::new($module_acl_categories).unwrap();
                if let Some(RM_AddACLCategory) = raw::RedisModule_AddACLCategory {
                    if RM_AddACLCategory(ctx, categories.as_ptr()) == raw::Status::Err as c_int {
                        raw::redis_log(ctx, &format!("Error: failed to add ACL categories `{}`", $module_acl_categories));
                        return raw::Status::Err as c_int;
                    }
                } else {
                    raw::redis_log(ctx, "Warning: Redis version does not support adding new ACL categories");
                }
            )*

            $(
                $crate::redis_command!(ctx, $name, $command, $flags, $firstkey, $lastkey, $keystep, $mandatory_command_acl_categories $(, $optional_command_acl_categories)?);
            )*

            if $crate::commands::register_commands(&context) == raw::Status::Err {
                return raw::Status::Err as c_int;
            }

            if let Err(e) = $crate::defrag::register_defrag_functions(&context) {
                context.log_warning(&format!("{e}"));
                return raw::Status::Err as c_int;
            }

            $(
                $(
                    $crate::redis_event_handler!(ctx, $(raw::NotifyEvent::$event_type |)+ raw::NotifyEvent::empty(), $event_handler);
                )*
            )?

            $(
                $(
                    $(
                        let default = if $use_module_args {
                            match get_i64_default_config_value(&args, $i64_configuration_name, $i64_default) {
                                Ok(v) => v,
                                Err(e) => {
                                    context.log_warning(&format!("{e}"));
                                    return raw::Status::Err as c_int;
                                }
                            }
                        } else {
                            $i64_default
                        };
                        register_i64_configuration(&context, $i64_configuration_name, $i64_configuration_val, default, $i64_min, $i64_max, $i64_flags_options, $i64_on_changed);
                    )*
                )?
                $(
                    $(
                        let default = if $use_module_args {
                            match get_string_default_config_value(&args, $string_configuration_name, $string_default) {
                                Ok(v) => v,
                                Err(e) => {
                                    context.log_warning(&format!("{e}"));
                                    return raw::Status::Err as c_int;
                                }
                            }
                        } else {
                            $string_default
                        };
                        register_string_configuration(&context, $string_configuration_name, $string_configuration_val, default, $string_flags_options, $string_on_changed);
                    )*
                )?
                $(
                    $(
                        let default = if $use_module_args {
                            match get_bool_default_config_value(&args, $bool_configuration_name, $bool_default) {
                                Ok(v) => v,
                                Err(e) => {
                                    context.log_warning(&format!("{e}"));
                                    return raw::Status::Err as c_int;
                                }
                            }
                        } else {
                            $bool_default
                        };
                        register_bool_configuration(&context, $bool_configuration_name, $bool_configuration_val, default, $bool_flags_options, $bool_on_changed);
                    )*
                )?
                $(
                    $(
                        let default = if $use_module_args {
                            match get_enum_default_config_value(&args, $enum_configuration_name, $enum_default) {
                                Ok(v) => v,
                                Err(e) => {
                                    context.log_warning(&format!("{e}"));
                                    return raw::Status::Err as c_int;
                                }
                            }
                        } else {
                            $enum_default
                        };
                        register_enum_configuration(&context, $enum_configuration_name, $enum_configuration_val, default, $enum_flags_options, $enum_on_changed);
                    )*
                )?
                raw::RedisModule_LoadConfigs.unwrap()(ctx);

                $(
                    $crate::redis_command!(ctx, $module_config_get_command, |ctx, args: Vec<RedisString>| {
                        module_config_get(ctx, args, $module_name)
                    }, "", 0, 0, 0, "");
                )?

                $(
                    $crate::redis_command!(ctx, $module_config_set_command, |ctx, args: Vec<RedisString>| {
                        module_config_set(ctx, args, $module_name)
                    }, "", 0, 0, 0, "");
                )?
            )?

            raw::register_info_function(ctx, Some(__info_func));

            if let Err(e) = register_server_events(&context) {
                context.log_warning(&format!("{e}"));
                return raw::Status::Err as c_int;
            }

            $(
                if $init_func(&context, &args) == $crate::Status::Err {
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
