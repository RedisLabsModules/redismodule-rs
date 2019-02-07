use std::ptr;
use std::string;
use std::os::raw::c_void;

use libc::size_t;

use crate::raw;
use crate::error::Error;
use crate::RedisString;
use crate::native_types::RedisType;
use crate::from_byte_string;

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

    /// Detects whether the key pointer given to us by Redis is null.
    pub fn is_null(&self) -> bool {
        let null_key: *mut raw::RedisModuleKey = ptr::null_mut();
        self.key_inner == null_key
    }

    pub fn read(&self) -> Result<Option<String>, Error> {
        let val = if self.is_null() {
            None
        } else {
            Some(read_key(self.key_inner)?)
        };
        Ok(val)
    }

    pub fn verify_and_get_type(&self, redis_type: &RedisType) -> Result<raw::KeyType, Error> {
        raw::verify_and_get_type(
            self.ctx,
            self.key_inner,
            *redis_type.raw_type.borrow_mut(),
        )
    }

    pub fn get_value(&self) -> *mut c_void {
        raw::module_type_get_value(self.key_inner)
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
        let key_inner =
            raw::open_key(ctx, key_str.inner, to_raw_mode(KeyMode::ReadWrite));
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
    pub fn is_empty(&self) -> Result<bool, Error> {
        match self.read()? {
            Some(s) => match s.as_str() {
                "" => Ok(true),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    pub fn read(&self) -> Result<Option<String>, Error> {
        Ok(Some(read_key(self.key_inner)?))
    }

    pub fn set_expire(&self, expire: time::Duration) -> Result<(), Error> {
        match raw::set_expire(self.key_inner, expire.num_milliseconds()) {
            raw::Status::Ok => Ok(()),

            // Error may occur if the key wasn't open for writing or is an
            // empty key.
            raw::Status::Err => Err(error!("Error while setting key expire")),
        }
    }

    pub fn write(&self, val: &str) -> Result<(), Error> {
        let val_str = RedisString::create(self.ctx, val);
        match raw::string_set(self.key_inner, val_str.inner) {
            raw::Status::Ok => Ok(()),
            raw::Status::Err => Err(error!("Error while setting key")),
        }
    }

    pub fn verify_and_get_type(&self, redis_type: &RedisType) -> Result<raw::KeyType, Error> {
        raw::verify_and_get_type(
            self.ctx,
            self.key_inner,
            *redis_type.raw_type.borrow_mut(),
        )
    }

    pub fn get_value(&self) -> *mut c_void {
        raw::module_type_get_value(self.key_inner)
    }

    pub fn set_value(&self, redis_type: &RedisType, value: *mut c_void) -> Result<(), Error> {
        raw::module_type_set_value(
            self.key_inner,
            *redis_type.raw_type.borrow_mut(),
            value,
        ).into()
    }
}

impl From<raw::Status> for Result<(), Error> {
    fn from(s: raw::Status) -> Self {
        match s {
            raw::Status::Ok => Ok(()),
            raw::Status::Err => Err(Error::generic("Generic error")),
        }
    }
}


impl Drop for RedisKeyWritable {
    // Frees resources appropriately as a RedisKey goes out of scope.
    fn drop(&mut self) {
        raw::close_key(self.key_inner);
    }
}

fn read_key(key: *mut raw::RedisModuleKey) -> Result<String, string::FromUtf8Error> {
    let mut length: size_t = 0;
    from_byte_string(
        raw::string_dma(key, &mut length, raw::KeyMode::READ),
        length,
    )
}

fn to_raw_mode(mode: KeyMode) -> raw::KeyMode {
    match mode {
        KeyMode::Read => raw::KeyMode::READ,
        KeyMode::ReadWrite => raw::KeyMode::READ | raw::KeyMode::WRITE,
    }
}
