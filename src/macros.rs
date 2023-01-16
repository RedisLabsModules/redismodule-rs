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

            raw::register_info_function(ctx, Some(__info_func));

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

/// Gets one or more fields from a hash value, and returns them in a
/// `HashMap<String, RedisString>`. The field names may be provided as string slices
/// or as `String` objects. The returned HashMap will contain an entry for each field
/// that exists in the key. If the key itself does not exist, the HashMap will be empty.
///
/// # Examples
///
/// ```
/// let k: RedisKey = ctx.open_key("config");
/// let cfg: HashMap<String, RedisString> =
///     hash_get!(k, "hostname", "port");
/// let hostname: &str = cfg.get("hostname")
///     .map_or("localhost", |rs| rs.try_as_str().unwrap());
/// let port: &str = cfg.get("port")
///     .map_or("443", |rs| rs.try_as_str().unwrap());
/// ```
#[macro_export]
macro_rules! hash_get {
    (@replace_expr $_t:tt $sub:expr) => { $sub };
    (@count_tts $($tts:tt)*) => { 0usize $(+ hash_get!(@replace_expr $tts 1usize))* };

    (@call ($key:expr) ($iter:ident) () -> ($($body:tt)*)) => {
        unsafe {
            use redis_module::redisraw::bindings::{RedisModule_HashGet, REDISMODULE_HASH_CFIELDS};
            use std::ffi::CString;

            RedisModule_HashGet.unwrap()(
                $key,
                REDISMODULE_HASH_CFIELDS as i32,
                $($body)*
                std::ptr::null::<std::os::raw::c_char>()
            )
        }
    };
    (@call ($key:expr) ($iter:ident) ($field:tt $($tail:tt)*) -> ($($body:tt)*)) => {
        hash_get!(@call ($key) ($iter) ($($tail)*) ->
                    ($($body)*
                     CString::new($field).unwrap().as_ptr(),
                     $iter.next().unwrap(),)
                 )
    };

    ($key:ident, $($field:tt),*) => {
        if !$key.is_null() {
            use redis_module::raw::{RedisModuleString, Status};
            use redis_module::redisraw::bindings::{RedisModule_HashGet, REDISMODULE_HASH_CFIELDS};
            use redis_module::{RedisString, RedisError};
            use std::collections::HashMap;

            const LEN: usize = hash_get!(@count_tts $($field)*);
            let mut values: [*mut RedisModuleString; LEN] = [std::ptr::null_mut(); LEN];
            let res = Status::from({
                let mut ivalues = values.iter_mut();
                let key_inner = unsafe { $key.get_inner() };
                hash_get!(@call (key_inner) (ivalues) ($($field)*) -> ())
            });
            if res == Status::Ok {
                let mut map: HashMap<String, RedisString> =
                        HashMap::with_capacity(LEN);
                {
                    let key_ctx = unsafe { $key.get_ctx() };
                    let mut ivalues = values.iter_mut();
                    unsafe {
                        $(
                            if let Some(p) = ivalues.next().unwrap().as_mut() {
                                map.insert($field.to_string(), RedisString::new(key_ctx, p));
                            }
                        )*
                    }
                }
                Ok(map)
            } else {
                Err(RedisError::Str("ERR key is not a hash value"))
            }
        } else {
            use redis_module::RedisString;
            Ok(HashMap::<String, RedisString>::with_capacity(0))
        }
    };
}
