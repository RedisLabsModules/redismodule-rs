use libc::size_t;
use std::convert::TryFrom;
use std::os::raw::c_void;
use std::ptr;
use std::str::Utf8Error;
use std::time::Duration;

use crate::from_byte_string;
use crate::native_types::RedisType;
use crate::raw;
use crate::redismodule::REDIS_OK;
use crate::RedisError;
use crate::RedisResult;
use crate::RedisString;

/// `RedisKey` is an abstraction over a Redis key that allows readonly
/// operations.
///
/// Its primary function is to ensure the proper deallocation of resources when
/// it goes out of scope. Redis normally requires that keys be managed manually
/// by explicitly freeing them when you're done. This can be a risky prospect,
/// especially with mechanics like Rust's `?` operator, so we ensure fault-free
/// operation through the use of the Drop trait.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyMode {
    Read,
    ReadWrite,
}

#[derive(Debug)]
pub struct RedisKey {
    ctx: *mut raw::RedisModuleCtx,
    key_inner: *mut raw::RedisModuleKey,
    key_str: RedisString,
}

impl RedisKey {
    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &str) -> RedisKey {
        let key_str = RedisString::create(ctx, key);
        let key_inner = raw::open_key(ctx, key_str.inner, to_raw_mode(KeyMode::Read));
        RedisKey {
            ctx,
            key_inner,
            key_str,
        }
    }

    pub fn get_value<T>(&self, redis_type: &RedisType) -> Result<Option<&T>, RedisError> {
        verify_type(self.key_inner, redis_type)?;

        let value =
            unsafe { raw::RedisModule_ModuleTypeGetValue.unwrap()(self.key_inner) as *mut T };

        if value.is_null() {
            return Ok(None);
        }

        let value = unsafe { &*value };

        Ok(Some(value))
    }

    /// Detects whether the key pointer given to us by Redis is null.
    pub fn is_null(&self) -> bool {
        let null_key: *mut raw::RedisModuleKey = ptr::null_mut();
        self.key_inner == null_key
    }

    pub fn read(&self) -> Result<Option<String>, RedisError> {
        let val = if self.is_null() {
            None
        } else {
            Some(read_key(self.key_inner)?)
        };
        Ok(val)
    }

    pub fn hash_get(&self, field: &str) -> Result<Option<RedisString>, RedisError> {
        let val = if self.is_null() {
            None
        } else {
            hash_get_key(self.ctx, self.key_inner, field)
        };
        Ok(val)
    }
}

impl Drop for RedisKey {
    // Frees resources appropriately as a RedisKey goes out of scope.
    fn drop(&mut self) {
        raw::close_key(self.key_inner);
    }
}

/// `RedisKeyWritable` is an abstraction over a Redis key that allows read and
/// write operations.
pub struct RedisKeyWritable {
    ctx: *mut raw::RedisModuleCtx,
    key_inner: *mut raw::RedisModuleKey,

    // The Redis string
    //
    // This field is needed on the struct so that its Drop implementation gets
    // called when it goes out of scope.
    #[allow(dead_code)]
    key_str: RedisString,
}

impl RedisKeyWritable {
    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &str) -> RedisKeyWritable {
        let key_str = RedisString::create(ctx, key);
        let key_inner = raw::open_key(ctx, key_str.inner, to_raw_mode(KeyMode::ReadWrite));
        RedisKeyWritable {
            ctx,
            key_inner,
            key_str,
        }
    }

    /// Detects whether the value stored in a Redis key is empty.
    ///
    /// Note that an empty key can be reliably detected by looking for a null
    /// as you open the key in read mode, but when asking for write Redis
    /// returns a non-null pointer to allow us to write to even an empty key,
    /// so we have to check the key's value instead.
    /*
    fn is_empty_old(&self) -> Result<bool, Error> {
        match self.read()? {
            Some(s) => match s.as_str() {
                "" => Ok(true),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }
    */

    pub fn read(&self) -> Result<Option<String>, RedisError> {
        Ok(Some(read_key(self.key_inner)?))
    }

    pub fn hash_set(&self, field: &str, value: RedisString) -> raw::Status {
        raw::hash_set(self.key_inner, field, value.inner)
    }

    pub fn hash_get(&self, field: &str) -> Result<Option<RedisString>, RedisError> {
        Ok(hash_get_key(self.ctx, self.key_inner, field))
    }

    pub fn set_expire(&self, expire: Duration) -> RedisResult {
        let exp_millis = expire.as_millis();

        let exp_time = i64::try_from(exp_millis).map_err(|_| {
            RedisError::String(format!(
                "Error expire duration {} is not allowed",
                exp_millis
            ))
        })?;

        match raw::set_expire(self.key_inner, exp_time) {
            raw::Status::Ok => REDIS_OK,

            // Error may occur if the key wasn't open for writing or is an
            // empty key.
            raw::Status::Err => Err(RedisError::Str("Error while setting key expire")),
        }
    }

    pub fn write(&self, val: &str) -> RedisResult {
        let val_str = RedisString::create(self.ctx, val);
        match raw::string_set(self.key_inner, val_str.inner) {
            raw::Status::Ok => REDIS_OK,
            raw::Status::Err => Err(RedisError::Str("Error while setting key")),
        }
    }

    pub fn delete(&self) -> RedisResult {
        unsafe { raw::RedisModule_DeleteKey.unwrap()(self.key_inner) };
        REDIS_OK
    }

    pub fn is_empty(&self) -> bool {
        use raw::KeyType;

        let key_type: KeyType = unsafe { raw::RedisModule_KeyType.unwrap()(self.key_inner) }.into();

        key_type == KeyType::Empty
    }

    pub fn get_value<T>(&self, redis_type: &RedisType) -> Result<Option<&mut T>, RedisError> {
        verify_type(self.key_inner, redis_type)?;
        let value =
            unsafe { raw::RedisModule_ModuleTypeGetValue.unwrap()(self.key_inner) as *mut T };

        if value.is_null() {
            return Ok(None);
        }

        let value = unsafe { &mut *value };
        Ok(Some(value))
    }

    pub fn set_value<T>(&self, redis_type: &RedisType, value: T) -> Result<(), RedisError> {
        verify_type(self.key_inner, redis_type)?;
        let value = Box::into_raw(Box::new(value)) as *mut c_void;
        let status: raw::Status = unsafe {
            raw::RedisModule_ModuleTypeSetValue.unwrap()(
                self.key_inner,
                *redis_type.raw_type.borrow(),
                value,
            )
        }
        .into();

        status.into()
    }
}

impl From<raw::Status> for Result<(), RedisError> {
    fn from(s: raw::Status) -> Self {
        match s {
            raw::Status::Ok => Ok(()),
            raw::Status::Err => Err(RedisError::String("Generic error".to_string())),
        }
    }
}

impl Drop for RedisKeyWritable {
    // Frees resources appropriately as a RedisKey goes out of scope.
    fn drop(&mut self) {
        raw::close_key(self.key_inner);
    }
}

fn read_key(key: *mut raw::RedisModuleKey) -> Result<String, Utf8Error> {
    let mut length: size_t = 0;
    from_byte_string(
        raw::string_dma(key, &mut length, raw::KeyMode::READ),
        length,
    )
}

fn hash_get_key(
    ctx: *mut raw::RedisModuleCtx,
    key: *mut raw::RedisModuleKey,
    field: &str,
) -> Option<RedisString> {
    let res = raw::hash_get(key, field);
    if res.is_null() {
        None
    } else {
        Some(RedisString::new(ctx, res))
    }
}

fn to_raw_mode(mode: KeyMode) -> raw::KeyMode {
    match mode {
        KeyMode::Read => raw::KeyMode::READ,
        KeyMode::ReadWrite => raw::KeyMode::READ | raw::KeyMode::WRITE,
    }
}

fn verify_type(key_inner: *mut raw::RedisModuleKey, redis_type: &RedisType) -> RedisResult {
    use raw::KeyType;

    let key_type: KeyType = unsafe { raw::RedisModule_KeyType.unwrap()(key_inner) }.into();

    if key_type != KeyType::Empty {
        // The key exists; check its type
        let raw_type = unsafe { raw::RedisModule_ModuleTypeGetType.unwrap()(key_inner) };

        if raw_type != *redis_type.raw_type.borrow() {
            return Err(RedisError::String(format!(
                "Existing key has wrong Redis type"
            )));
        }
    }

    REDIS_OK
}
