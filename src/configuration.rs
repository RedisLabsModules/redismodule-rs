use crate::{raw, RedisGILGuard};
use crate::{Context, RedisError, RedisString};
use bitflags::bitflags;
use std::ffi::{c_char, c_int, c_longlong, c_void, CString};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;

bitflags! {
    pub struct ConfigurationFlags : c_int {
        /// The default flags for a config. This creates a config that can be modified after startup.
        const DEFAULT = raw::REDISMODULE_CONFIG_DEFAULT as c_int;

        /// This config can only be provided loading time.
        const IMMUTABLE = raw::REDISMODULE_CONFIG_IMMUTABLE as c_int;

        /// The value stored in this config is redacted from all logging.
        const SENSITIVE = raw::REDISMODULE_CONFIG_SENSITIVE as c_int;

        /// The name is hidden from `CONFIG GET` with pattern matching.
        const HIDDEN = raw::REDISMODULE_CONFIG_HIDDEN as c_int;

        /// This config will be only be modifiable based off the value of enable-protected-configs.
        const PROTECTED = raw::REDISMODULE_CONFIG_PROTECTED as c_int;

        /// This config is not modifiable while the server is loading data.
        const DENY_LOADING = raw::REDISMODULE_CONFIG_DENY_LOADING as c_int;

        /// For numeric configs, this config will convert data unit notations into their byte equivalent.
        const MEMORY = raw::REDISMODULE_CONFIG_MEMORY as c_int;

        /// For enum configs, this config will allow multiple entries to be combined as bit flags.
        const BITFLAGS = raw::REDISMODULE_CONFIG_BITFLAGS as c_int;
    }
}

#[macro_export]
macro_rules! enum_configuration {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident = $val:expr,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname = $val,)*
        }

        impl std::convert::TryFrom<i32> for $name {
            type Error = $crate::RedisError;

            fn try_from(v: i32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as i32 => Ok($name::$vname),)*
                    _ => Err($crate::RedisError::Str("Value is not supported")),
                }
            }
        }

        impl std::convert::From<$name> for i32 {
            fn from(val: $name) -> Self {
                val as i32
            }
        }

        impl EnumConfigurationValue for $name {
            fn get_options(&self) -> (Vec<String>, Vec<i32>) {
                (vec![$(stringify!($vname).to_string(),)*], vec![$($val,)*])
            }
        }

        impl Clone for $name {
            fn clone(&self) -> Self {
                match self {
                    $($name::$vname => $name::$vname,)*
                }
            }
        }
    }
}

pub trait ConfigurationValue<T>: Sync + Send {
    fn get(&self, ctx: &Context) -> T;
    fn set(&self, ctx: &Context, val: T) -> Result<(), RedisError>;
}

pub trait EnumConfigurationValue: TryFrom<i32, Error = RedisError> + Into<i32> + Clone {
    fn get_options(&self) -> (Vec<String>, Vec<i32>);
}

impl<T: Clone> ConfigurationValue<T> for RedisGILGuard<T> {
    fn get(&self, ctx: &Context) -> T {
        let value = self.lock(ctx);
        value.clone()
    }
    fn set(&self, ctx: &Context, val: T) -> Result<(), RedisError> {
        let mut value = self.lock(ctx);
        *value = val;
        Ok(())
    }
}

impl ConfigurationValue<i64> for AtomicI64 {
    fn get(&self, _ctx: &Context) -> i64 {
        self.load(Ordering::Relaxed)
    }
    fn set(&self, _ctx: &Context, val: i64) -> Result<(), RedisError> {
        self.store(val, Ordering::Relaxed);
        Ok(())
    }
}

impl ConfigurationValue<RedisString> for RedisGILGuard<String> {
    fn get(&self, ctx: &Context) -> RedisString {
        let value = self.lock(ctx);
        RedisString::create(None, &value)
    }
    fn set(&self, ctx: &Context, val: RedisString) -> Result<(), RedisError> {
        let mut value = self.lock(ctx);
        *value = val.try_as_str()?.to_string();
        Ok(())
    }
}

impl ConfigurationValue<RedisString> for Mutex<String> {
    fn get(&self, _ctx: &Context) -> RedisString {
        let value = self.lock().unwrap();
        RedisString::create(None, &value)
    }
    fn set(&self, _ctx: &Context, val: RedisString) -> Result<(), RedisError> {
        let mut value = self.lock().unwrap();
        *value = val.try_as_str()?.to_string();
        Ok(())
    }
}

impl ConfigurationValue<bool> for AtomicBool {
    fn get(&self, _ctx: &Context) -> bool {
        self.load(Ordering::Relaxed)
    }
    fn set(&self, _ctx: &Context, val: bool) -> Result<(), RedisError> {
        self.store(val, Ordering::Relaxed);
        Ok(())
    }
}

extern "C" fn i64_configuration_set<T: ConfigurationValue<i64>>(
    _name: *const c_char,
    val: c_longlong,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    if let Err(e) = variable.set(&Context::dummy(), val) {
        let error_msg = RedisString::create(None, &e.to_string());
        unsafe { *err = error_msg.take() };
        return raw::REDISMODULE_ERR as i32;
    }
    raw::REDISMODULE_OK as i32
}

extern "C" fn i64_configuration_get<T: ConfigurationValue<i64>>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_longlong {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    variable.get(&Context::dummy())
}

pub fn register_i64_configuration<T: ConfigurationValue<i64>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: i64,
    min: i64,
    max: i64,
    flags: ConfigurationFlags,
) {
    let name = CString::new(name).unwrap();
    unsafe {
        raw::RedisModule_RegisterNumericConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default,
            flags.bits() as u32,
            min,
            max,
            Some(i64_configuration_get::<T>),
            Some(i64_configuration_set::<T>),
            None,
            variable as *const T as *mut c_void,
        );
    }
}

extern "C" fn string_configuration_set<T: ConfigurationValue<RedisString>>(
    _name: *const c_char,
    val: *mut raw::RedisModuleString,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let new_val = RedisString::new(None, val);
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    if let Err(e) = variable.set(&Context::dummy(), new_val) {
        let error_msg = RedisString::create(None, &e.to_string());
        unsafe { *err = error_msg.take() };
        return raw::REDISMODULE_ERR as i32;
    }
    raw::REDISMODULE_OK as i32
}

extern "C" fn string_configuration_get<T: ConfigurationValue<RedisString>>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> *mut raw::RedisModuleString {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    variable.get(&Context::dummy()).take()
}

pub fn register_string_configuration<T: ConfigurationValue<RedisString>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: &str,
    flags: ConfigurationFlags,
) {
    let name = CString::new(name).unwrap();
    let default = CString::new(default).unwrap();
    unsafe {
        raw::RedisModule_RegisterStringConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default.as_ptr(),
            flags.bits() as u32,
            Some(string_configuration_get::<T>),
            Some(string_configuration_set::<T>),
            None,
            variable as *const T as *mut c_void,
        );
    }
}

extern "C" fn bool_configuration_set<T: ConfigurationValue<bool>>(
    _name: *const c_char,
    val: i32,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    if let Err(e) = variable.set(&Context::dummy(), val != 0) {
        let error_msg = RedisString::create(None, &e.to_string());
        unsafe { *err = error_msg.take() };
        return raw::REDISMODULE_ERR as i32;
    }
    raw::REDISMODULE_OK as i32
}

extern "C" fn bool_configuration_get<T: ConfigurationValue<bool>>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    variable.get(&Context::dummy()) as i32
}

pub fn register_bool_configuration<T: ConfigurationValue<bool>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: bool,
    flags: ConfigurationFlags,
) {
    let name = CString::new(name).unwrap();
    unsafe {
        raw::RedisModule_RegisterBoolConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default as i32,
            flags.bits() as u32,
            Some(bool_configuration_get::<T>),
            Some(bool_configuration_set::<T>),
            None,
            variable as *const T as *mut c_void,
        );
    }
}

extern "C" fn enum_configuration_set<G: EnumConfigurationValue, T: ConfigurationValue<G>>(
    _name: *const c_char,
    val: i32,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    if let Err(e) = val
        .try_into()
        .and_then(|v| variable.set(&Context::dummy(), v))
    {
        let error_msg = RedisString::create(None, &e.to_string());
        unsafe { *err = error_msg.take() };
        return raw::REDISMODULE_ERR as i32;
    }
    raw::REDISMODULE_OK as i32
}

extern "C" fn enum_configuration_get<G: EnumConfigurationValue, T: ConfigurationValue<G>>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let variable = unsafe { &*(privdata as *const T) };
    // we know the GIL is held so it is safe to use Context::dummy().
    variable.get(&Context::dummy()).into()
}

pub fn register_enum_configuration<G: EnumConfigurationValue, T: ConfigurationValue<G>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: G,
    flags: ConfigurationFlags,
) {
    let name = CString::new(name).unwrap();
    let (names, vals) = default.get_options();
    assert_eq!(names.len(), vals.len());
    let names: Vec<CString> = names
        .into_iter()
        .map(|v| CString::new(v).unwrap())
        .collect();
    unsafe {
        raw::RedisModule_RegisterEnumConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default.into(),
            flags.bits() as u32,
            names
                .iter()
                .map(|v| v.as_ptr())
                .collect::<Vec<*const c_char>>()
                .as_mut_ptr(),
            vals.as_ptr(),
            names.len() as i32,
            Some(enum_configuration_get::<G, T>),
            Some(enum_configuration_set::<G, T>),
            None,
            variable as *const T as *mut c_void,
        );
    }
}
