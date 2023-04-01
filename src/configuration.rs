use crate::context::thread_safe::RedisLockIndicator;
use crate::{raw, RedisGILGuard};
use crate::{Context, RedisError, RedisString};
use bitflags::bitflags;
use std::ffi::{c_char, c_int, c_longlong, c_void, CStr, CString};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Mutex;

bitflags! {
    /// Configuration options
    pub struct ConfigurationFlags : u32 {
        /// The default flags for a config. This creates a config that can be modified after startup.
        const DEFAULT = raw::REDISMODULE_CONFIG_DEFAULT;

        /// This config can only be provided loading time.
        const IMMUTABLE = raw::REDISMODULE_CONFIG_IMMUTABLE;

        /// The value stored in this config is redacted from all logging.
        const SENSITIVE = raw::REDISMODULE_CONFIG_SENSITIVE;

        /// The name is hidden from `CONFIG GET` with pattern matching.
        const HIDDEN = raw::REDISMODULE_CONFIG_HIDDEN;

        /// This config will be only be modifiable based off the value of enable-protected-configs.
        const PROTECTED = raw::REDISMODULE_CONFIG_PROTECTED;

        /// This config is not modifiable while the server is loading data.
        const DENY_LOADING = raw::REDISMODULE_CONFIG_DENY_LOADING;

        /// For numeric configs, this config will convert data unit notations into their byte equivalent.
        const MEMORY = raw::REDISMODULE_CONFIG_MEMORY;

        /// For enum configs, this config will allow multiple entries to be combined as bit flags.
        const BITFLAGS = raw::REDISMODULE_CONFIG_BITFLAGS;
    }
}

#[macro_export]
macro_rules! enum_configuration {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident = $val:expr,)*
    }) => {
        use $crate::configuration::EnumConfigurationValue;
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

/// [`ConfigurationContext`] is used as a special context that indicate that we are
/// running with the Redis GIL is held but we should not perform all the regular
/// operation we can perfrom on the regular Context.
pub struct ConfigurationContext {
    _dummy: usize, // We set some none public vairable here so user will not be able to construct such object
}

impl ConfigurationContext {
    fn new() -> ConfigurationContext {
        ConfigurationContext { _dummy: 0 }
    }
}

unsafe impl RedisLockIndicator for ConfigurationContext {}

pub trait ConfigurationValue<T>: Sync + Send {
    fn get(&self, ctx: &ConfigurationContext) -> T;
    fn set(&self, ctx: &ConfigurationContext, val: T) -> Result<(), RedisError>;
}

pub trait EnumConfigurationValue: TryFrom<i32, Error = RedisError> + Into<i32> + Clone {
    fn get_options(&self) -> (Vec<String>, Vec<i32>);
}

impl<T: Clone> ConfigurationValue<T> for RedisGILGuard<T> {
    fn get(&self, ctx: &ConfigurationContext) -> T {
        let value = self.lock(ctx);
        value.clone()
    }
    fn set(&self, ctx: &ConfigurationContext, val: T) -> Result<(), RedisError> {
        let mut value = self.lock(ctx);
        *value = val;
        Ok(())
    }
}

impl<T: Clone + Send> ConfigurationValue<T> for Mutex<T> {
    fn get(&self, _ctx: &ConfigurationContext) -> T {
        let value = self.lock().unwrap();
        value.clone()
    }
    fn set(&self, _ctx: &ConfigurationContext, val: T) -> Result<(), RedisError> {
        let mut value = self.lock().unwrap();
        *value = val;
        Ok(())
    }
}

impl ConfigurationValue<i64> for AtomicI64 {
    fn get(&self, _ctx: &ConfigurationContext) -> i64 {
        self.load(Ordering::Relaxed)
    }
    fn set(&self, _ctx: &ConfigurationContext, val: i64) -> Result<(), RedisError> {
        self.store(val, Ordering::Relaxed);
        Ok(())
    }
}

impl ConfigurationValue<RedisString> for RedisGILGuard<String> {
    fn get(&self, ctx: &ConfigurationContext) -> RedisString {
        let value = self.lock(ctx);
        RedisString::create(None, value.as_str())
    }
    fn set(&self, ctx: &ConfigurationContext, val: RedisString) -> Result<(), RedisError> {
        let mut value = self.lock(ctx);
        *value = val.try_as_str()?.to_string();
        Ok(())
    }
}

impl ConfigurationValue<RedisString> for Mutex<String> {
    fn get(&self, _ctx: &ConfigurationContext) -> RedisString {
        let value = self.lock().unwrap();
        RedisString::create(None, value.as_str())
    }
    fn set(&self, _ctx: &ConfigurationContext, val: RedisString) -> Result<(), RedisError> {
        let mut value = self.lock().unwrap();
        *value = val.try_as_str()?.to_string();
        Ok(())
    }
}

impl ConfigurationValue<bool> for AtomicBool {
    fn get(&self, _ctx: &ConfigurationContext) -> bool {
        self.load(Ordering::Relaxed)
    }
    fn set(&self, _ctx: &ConfigurationContext, val: bool) -> Result<(), RedisError> {
        self.store(val, Ordering::Relaxed);
        Ok(())
    }
}

type OnUpdatedCallback<T> = Box<dyn Fn(&ConfigurationContext, &str, &'static T)>;

struct ConfigrationPrivateData<G, T: ConfigurationValue<G> + 'static> {
    variable: &'static T,
    on_changed: Option<OnUpdatedCallback<T>>,
    phantom: PhantomData<G>,
}

impl<G, T: ConfigurationValue<G> + 'static> ConfigrationPrivateData<G, T> {
    fn set_val(&self, name: *const c_char, val: G, err: *mut *mut raw::RedisModuleString) -> c_int {
        // we know the GIL is held so it is safe to use Context::dummy().
        let configuration_ctx = ConfigurationContext::new();
        if let Err(e) = self.variable.set(&configuration_ctx, val) {
            let error_msg = RedisString::create(None, e.to_string().as_str());
            unsafe { *err = error_msg.take() };
            return raw::REDISMODULE_ERR as i32;
        }
        let c_str_name = unsafe { CStr::from_ptr(name) };
        self.on_changed.as_ref().map(|v| {
            v(
                &configuration_ctx,
                c_str_name.to_str().unwrap(),
                self.variable,
            )
        });
        raw::REDISMODULE_OK as i32
    }

    fn get_val(&self) -> G {
        self.variable.get(&ConfigurationContext::new())
    }
}

extern "C" fn i64_configuration_set<T: ConfigurationValue<i64> + 'static>(
    name: *const c_char,
    val: c_longlong,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<i64, T>) };
    private_data.set_val(name, val, err)
}

extern "C" fn i64_configuration_get<T: ConfigurationValue<i64> + 'static>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_longlong {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<i64, T>) };
    private_data.get_val()
}

pub fn register_i64_configuration<T: ConfigurationValue<i64>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: i64,
    min: i64,
    max: i64,
    flags: ConfigurationFlags,
    on_changed: Option<OnUpdatedCallback<T>>,
) {
    let name = CString::new(name).unwrap();
    let config_private_data = ConfigrationPrivateData {
        variable: variable,
        on_changed: on_changed,
        phantom: PhantomData::<i64>,
    };
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
            Box::into_raw(Box::new(config_private_data)) as *mut c_void,
        );
    }
}

extern "C" fn string_configuration_set<T: ConfigurationValue<RedisString> + 'static>(
    name: *const c_char,
    val: *mut raw::RedisModuleString,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let new_val = RedisString::new(None, val);
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<RedisString, T>) };
    private_data.set_val(name, new_val, err)
}

extern "C" fn string_configuration_get<T: ConfigurationValue<RedisString> + 'static>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> *mut raw::RedisModuleString {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<RedisString, T>) };
    // we know the GIL is held so it is safe to use Context::dummy().
    private_data
        .variable
        .get(&ConfigurationContext::new())
        .take()
}

pub fn register_string_configuration<T: ConfigurationValue<RedisString>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: &str,
    flags: ConfigurationFlags,
    on_changed: Option<OnUpdatedCallback<T>>,
) {
    let name = CString::new(name).unwrap();
    let default = CString::new(default).unwrap();
    let config_private_data = ConfigrationPrivateData {
        variable: variable,
        on_changed: on_changed,
        phantom: PhantomData::<RedisString>,
    };
    unsafe {
        raw::RedisModule_RegisterStringConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default.as_ptr(),
            flags.bits() as u32,
            Some(string_configuration_get::<T>),
            Some(string_configuration_set::<T>),
            None,
            Box::into_raw(Box::new(config_private_data)) as *mut c_void,
        );
    }
}

extern "C" fn bool_configuration_set<T: ConfigurationValue<bool> + 'static>(
    name: *const c_char,
    val: i32,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<bool, T>) };
    private_data.set_val(name, val != 0, err)
}

extern "C" fn bool_configuration_get<T: ConfigurationValue<bool> + 'static>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<bool, T>) };
    private_data.get_val() as i32
}

pub fn register_bool_configuration<T: ConfigurationValue<bool>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: bool,
    flags: ConfigurationFlags,
    on_changed: Option<OnUpdatedCallback<T>>,
) {
    let name = CString::new(name).unwrap();
    let config_private_data = ConfigrationPrivateData {
        variable: variable,
        on_changed: on_changed,
        phantom: PhantomData::<bool>,
    };
    unsafe {
        raw::RedisModule_RegisterBoolConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default as i32,
            flags.bits() as u32,
            Some(bool_configuration_get::<T>),
            Some(bool_configuration_set::<T>),
            None,
            Box::into_raw(Box::new(config_private_data)) as *mut c_void,
        );
    }
}

extern "C" fn enum_configuration_set<
    G: EnumConfigurationValue,
    T: ConfigurationValue<G> + 'static,
>(
    name: *const c_char,
    val: i32,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<G, T>) };
    let val: Result<G, _> = val.try_into();
    match val {
        Ok(val) => private_data.set_val(name, val, err),
        Err(e) => {
            let error_msg = RedisString::create(None, e.to_string().as_str());
            unsafe { *err = error_msg.take() };
            raw::REDISMODULE_ERR as i32
        }
    }
}

extern "C" fn enum_configuration_get<
    G: EnumConfigurationValue,
    T: ConfigurationValue<G> + 'static,
>(
    _name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let private_data = unsafe { &*(privdata as *const ConfigrationPrivateData<G, T>) };
    private_data.get_val().into()
}

pub fn register_enum_configuration<G: EnumConfigurationValue, T: ConfigurationValue<G>>(
    ctx: &Context,
    name: &str,
    variable: &'static T,
    default: G,
    flags: ConfigurationFlags,
    on_changed: Option<OnUpdatedCallback<T>>,
) {
    let name = CString::new(name).unwrap();
    let (names, vals) = default.get_options();
    assert_eq!(names.len(), vals.len());
    let names: Vec<CString> = names
        .into_iter()
        .map(|v| CString::new(v).unwrap())
        .collect();
    let config_private_data = ConfigrationPrivateData {
        variable: variable,
        on_changed: on_changed,
        phantom: PhantomData::<G>,
    };
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
            Box::into_raw(Box::new(config_private_data)) as *mut c_void,
        );
    }
}

pub fn apply_module_args_as_configuration(
    ctx: &Context,
    args: &[RedisString],
) -> Result<(), RedisError> {
    if args.len() == 0 {
        return Ok(());
    }
    if args.len() % 2 != 0 {
        return Err(RedisError::Str(
            "Arguments lenght is not devided by 2 (require to be read as module configuration).",
        ));
    }
    let mut args = args.to_vec();
    args.insert(0, ctx.create_string("set"));
    ctx.call(
        "config",
        args.iter().collect::<Vec<&RedisString>>().as_slice(),
    )?;
    Ok(())
}
