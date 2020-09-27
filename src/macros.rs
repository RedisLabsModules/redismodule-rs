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
        #[no_mangle]
        #[allow(non_snake_case)]
        pub extern "C" fn RedisModule_OnLoad(
            ctx: *mut $crate::raw::RedisModuleCtx,
            _argv: *mut *mut $crate::raw::RedisModuleString,
            _argc: std::os::raw::c_int,
        ) -> std::os::raw::c_int {
            use std::os::raw::{c_int, c_char};
            use std::ffi::CString;
            use std::slice;

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
