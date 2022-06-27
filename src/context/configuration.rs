use crate::context::Context;
use crate::raw;
use crate::{RedisError, RedisString};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_longlong, c_void};

pub struct ConfigFlags {
    flags: u32,
}

impl ConfigFlags {
    pub fn new() -> Self {
        ConfigFlags {
            flags: raw::REDISMODULE_CONFIG_DEFAULT,
        }
    }

    pub fn emmutable(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_IMMUTABLE;
        self
    }

    pub fn sensitive(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_SENSITIVE;
        self
    }

    pub fn hidden(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_HIDDEN;
        self
    }

    pub fn protected(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_PROTECTED;
        self
    }

    pub fn deny_loading(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_DENY_LOADING;
        self
    }

    pub fn memory(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_MEMORY;
        self
    }

    pub fn bit_flags(mut self) -> Self {
        self.flags |= raw::REDISMODULE_CONFIG_BITFLAGS;
        self
    }
}

pub trait RedisConfigCtx {
    fn name(&self) -> &'static str;
    fn apply(&self, ctx: &Context) -> Result<(), RedisError>;
    fn flags(&self) -> &ConfigFlags;
}

pub trait RedisStringConfigCtx: RedisConfigCtx {
    fn default(&self) -> Option<&'static str>;
    fn get(&self, name: &str) -> RedisString;
    fn set(&mut self, name: &str, value: RedisString) -> Result<(), RedisError>;
}

pub trait RedisBoolConfigCtx: RedisConfigCtx {
    fn default(&self) -> bool;
    fn get(&self, name: &str) -> bool;
    fn set(&mut self, name: &str, value: bool) -> Result<(), RedisError>;
}

pub trait RedisNumberConfigCtx: RedisConfigCtx {
    fn default(&self) -> i64;
    fn min(&self) -> i64;
    fn max(&self) -> i64;
    fn get(&self, name: &str) -> i64;
    fn set(&mut self, name: &str, value: i64) -> Result<(), RedisError>;
}

pub trait RedisEnumConfigCtx: RedisConfigCtx {
    fn default(&self) -> i32;
    fn values(&self) -> Vec<(&str, i32)>;
    fn get(&self, name: &str) -> i32;
    fn set(&mut self, name: &str, value: i32) -> Result<(), RedisError>;
}

extern "C" fn internal_string_get<C: RedisStringConfigCtx>(
    name: *const c_char,
    privdata: *mut c_void,
) -> *mut raw::RedisModuleString {
    let redis_config_ctx = unsafe { &*(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    let res = redis_config_ctx.get(name);
    res.inner
}

extern "C" fn inner_string_set<C: RedisStringConfigCtx>(
    name: *const c_char,
    val: *mut raw::RedisModuleString,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let redis_config_ctx = unsafe { &mut *(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    let new_val = RedisString::new(std::ptr::null_mut(), val);
    match redis_config_ctx.set(name, new_val) {
        Ok(_) => raw::REDISMODULE_OK as i32,
        Err(e) => {
            let err_msg = RedisString::create(
                std::ptr::null_mut(),
                &format!("Failed setting configuration value `{}`, {}", name, e),
            );
            unsafe {
                *err = err_msg.inner;
                raw::string_retain_string(std::ptr::null_mut(), *err);
            }
            raw::REDISMODULE_ERR as i32
        }
    }
}

extern "C" fn internal_bool_get<C: RedisBoolConfigCtx>(
    name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let redis_config_ctx = unsafe { &*(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    redis_config_ctx.get(name) as c_int
}

extern "C" fn inner_bool_set<C: RedisBoolConfigCtx>(
    name: *const c_char,
    val: c_int,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let redis_config_ctx = unsafe { &mut *(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    match redis_config_ctx.set(name, val > 0) {
        Ok(_) => raw::REDISMODULE_OK as i32,
        Err(e) => {
            let err_msg = RedisString::create(
                std::ptr::null_mut(),
                &format!("Failed setting configuration value `{}`, {}", name, e),
            );
            unsafe {
                *err = err_msg.inner;
                raw::string_retain_string(std::ptr::null_mut(), *err);
            }
            raw::REDISMODULE_ERR as i32
        }
    }
}

extern "C" fn internal_number_get<C: RedisNumberConfigCtx>(
    name: *const c_char,
    privdata: *mut c_void,
) -> c_longlong {
    let redis_config_ctx = unsafe { &*(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    redis_config_ctx.get(name)
}

extern "C" fn inner_number_set<C: RedisNumberConfigCtx>(
    name: *const c_char,
    val: c_longlong,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let redis_config_ctx = unsafe { &mut *(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    match redis_config_ctx.set(name, val) {
        Ok(_) => raw::REDISMODULE_OK as i32,
        Err(e) => {
            let err_msg = RedisString::create(
                std::ptr::null_mut(),
                &format!("Failed setting configuration value `{}`, {}", name, e),
            );
            unsafe {
                *err = err_msg.inner;
                raw::string_retain_string(std::ptr::null_mut(), *err);
            }
            raw::REDISMODULE_ERR as i32
        }
    }
}

extern "C" fn internal_enum_get<C: RedisEnumConfigCtx>(
    name: *const c_char,
    privdata: *mut c_void,
) -> c_int {
    let redis_config_ctx = unsafe { &*(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    redis_config_ctx.get(name)
}

extern "C" fn internal_enum_set<C: RedisEnumConfigCtx>(
    name: *const c_char,
    val: c_int,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let redis_config_ctx = unsafe { &mut *(privdata as *mut C) };
    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
    match redis_config_ctx.set(name, val) {
        Ok(_) => raw::REDISMODULE_OK as i32,
        Err(e) => {
            let err_msg = RedisString::create(
                std::ptr::null_mut(),
                &format!("Failed setting configuration value `{}`, {}", name, e),
            );
            unsafe {
                *err = err_msg.inner;
                raw::string_retain_string(std::ptr::null_mut(), *err);
            }
            raw::REDISMODULE_ERR as i32
        }
    }
}

extern "C" fn inner_apply<C: RedisConfigCtx>(
    ctx: *mut raw::RedisModuleCtx,
    privdata: *mut c_void,
    err: *mut *mut raw::RedisModuleString,
) -> c_int {
    let redis_config_ctx = unsafe { &*(privdata as *mut C) };
    let context = Context::new(ctx);
    match redis_config_ctx.apply(&context) {
        Ok(_) => raw::REDISMODULE_OK as i32,
        Err(e) => {
            let err_msg = RedisString::create(
                std::ptr::null_mut(),
                &format!("Failed apply configuration value, {}", e),
            );
            unsafe {
                *err = err_msg.inner;
                raw::string_retain_string(std::ptr::null_mut(), *err);
            }
            raw::REDISMODULE_ERR as i32
        }
    }
}

pub fn register_string_configuration<C: RedisStringConfigCtx>(
    ctx: &Context,
    redis_config_ctx: &C,
) -> Result<(), RedisError> {
    let name = match CString::new(redis_config_ctx.name()) {
        Ok(n) => n,
        Err(e) => return Err(RedisError::String(format!("{}", e))),
    };
    let default_val = match redis_config_ctx.default() {
        Some(d_v) => match CString::new(d_v) {
            Ok(res) => Some(res),
            Err(e) => return Err(RedisError::String(format!("{}", e))),
        },
        None => None,
    };
    let default_val_ptr = match default_val {
        Some(d_v) => d_v.as_ptr(),
        None => std::ptr::null_mut(),
    };
    let res = unsafe {
        raw::RedisModule_RegisterStringConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            default_val_ptr as *const c_char,
            redis_config_ctx.flags().flags,
            Some(internal_string_get::<C>),
            Some(inner_string_set::<C>),
            Some(inner_apply::<C>),
            redis_config_ctx as *const C as *mut c_void,
        )
    };

    if res != raw::REDISMODULE_OK as i32 {
        Err(RedisError::Str("Failed to register config"))
    } else {
        Ok(())
    }
}

pub fn register_bool_configuration<C: RedisBoolConfigCtx>(
    ctx: &Context,
    redis_config_ctx: &C,
) -> Result<(), RedisError> {
    let name = match CString::new(redis_config_ctx.name()) {
        Ok(n) => n,
        Err(e) => return Err(RedisError::String(format!("{}", e))),
    };
    let res = unsafe {
        raw::RedisModule_RegisterBoolConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            redis_config_ctx.default() as c_int,
            redis_config_ctx.flags().flags,
            Some(internal_bool_get::<C>),
            Some(inner_bool_set::<C>),
            Some(inner_apply::<C>),
            redis_config_ctx as *const C as *mut c_void,
        )
    };

    if res != raw::REDISMODULE_OK as i32 {
        Err(RedisError::Str("Failed to register config"))
    } else {
        Ok(())
    }
}

pub fn register_numeric_configuration<C: RedisNumberConfigCtx>(
    ctx: &Context,
    redis_config_ctx: &C,
) -> Result<(), RedisError> {
    let name = match CString::new(redis_config_ctx.name()) {
        Ok(n) => n,
        Err(e) => return Err(RedisError::String(format!("{}", e))),
    };
    let res = unsafe {
        raw::RedisModule_RegisterNumericConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            redis_config_ctx.default(),
            redis_config_ctx.flags().flags,
            redis_config_ctx.min(),
            redis_config_ctx.max(),
            Some(internal_number_get::<C>),
            Some(inner_number_set::<C>),
            Some(inner_apply::<C>),
            redis_config_ctx as *const C as *mut c_void,
        )
    };

    if res != raw::REDISMODULE_OK as i32 {
        Err(RedisError::Str("Failed to register config"))
    } else {
        Ok(())
    }
}

pub fn register_enum_configuration<C: RedisEnumConfigCtx>(
    ctx: &Context,
    redis_config_ctx: &C,
) -> Result<(), RedisError> {
    let name = match CString::new(redis_config_ctx.name()) {
        Ok(n) => n,
        Err(e) => return Err(RedisError::String(format!("{}", e))),
    };
    let values = redis_config_ctx.values();
    let mut enum_strings = Vec::new();
    let mut enum_values = Vec::new();
    for (enum_string, val) in values {
        enum_strings.push(CString::new(enum_string).unwrap());
        enum_values.push(val);
    }

    let res = unsafe {
        raw::RedisModule_RegisterEnumConfig.unwrap()(
            ctx.ctx,
            name.as_ptr(),
            redis_config_ctx.default() as i32,
            redis_config_ctx.flags().flags,
            enum_strings
                .iter()
                .map(|v| v.as_ptr())
                .collect::<Vec<*const c_char>>()
                .as_ptr() as *mut *const c_char,
            enum_values.as_ptr(),
            enum_strings.len() as i32,
            Some(internal_enum_get::<C>),
            Some(internal_enum_set::<C>),
            Some(inner_apply::<C>),
            redis_config_ctx as *const C as *mut c_void,
        )
    };

    if res != raw::REDISMODULE_OK as i32 {
        Err(RedisError::Str("Failed to register config"))
    } else {
        Ok(())
    }
}
