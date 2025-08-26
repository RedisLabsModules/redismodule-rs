use std::convert::TryFrom;
use std::ops::Deref;
use std::ops::DerefMut;
use std::os::raw::c_void;
use std::ptr;
use std::ptr::NonNull;
use std::time::Duration;

use libc::size_t;
use std::os::raw::c_int;

use raw::KeyType;

use crate::native_types::RedisType;
use crate::raw;
use crate::redismodule::REDIS_OK;
pub use crate::redisraw::bindings::*;
use crate::stream::StreamIterator;
use crate::RedisError;
use crate::RedisResult;
use crate::RedisString;
use bitflags::bitflags;

/// `RedisKey` is an abstraction over a Redis key that allows readonly
/// operations.
///
/// Its primary function is to ensure the proper deallocation of resources when
/// it goes out of scope. Redis normally requires that keys be managed manually
/// by explicitly freeing them when you're done. This can be a risky prospect,
/// especially with mechanics like Rust's `?` operator, so we ensure fault-free
/// operation through the use of the Drop trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyMode {
    Read,
    ReadWrite,
}

bitflags! {
    pub struct KeyFlags: c_int {
        /// Avoid touching the LRU/LFU of the key when opened.
        const NOTOUCH = REDISMODULE_OPEN_KEY_NOTOUCH as c_int;
        /// Don't trigger keyspace event on key misses.
        const NONOTIFY = REDISMODULE_OPEN_KEY_NONOTIFY as c_int;
        /// Don't update keyspace hits/misses counters.
        const NOSTATS = REDISMODULE_OPEN_KEY_NOSTATS as c_int;
        /// Avoid deleting lazy expired keys.
        const NOEXPIRE = REDISMODULE_OPEN_KEY_NOEXPIRE as c_int;
        /// Avoid any effects from fetching the key.
        const NOEFFECTS = REDISMODULE_OPEN_KEY_NOEFFECTS as c_int;
        /// Access lazy expire fields
        const ACCESS_EXPIRED = REDISMODULE_OPEN_KEY_ACCESS_EXPIRED as c_int;
    }
}

#[derive(Debug)]
pub struct RedisKey {
    pub(crate) ctx: *mut raw::RedisModuleCtx,
    pub(crate) key_inner: *mut raw::RedisModuleKey,
}

impl RedisKey {
    pub(crate) fn take(mut self) -> *mut raw::RedisModuleKey {
        let res = self.key_inner;
        self.key_inner = std::ptr::null_mut();
        res
    }

    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &RedisString) -> Self {
        let key_inner = raw::open_key(ctx, key.inner, to_raw_mode(KeyMode::Read));
        Self { ctx, key_inner }
    }

    pub(crate) fn open_with_flags(
        ctx: *mut raw::RedisModuleCtx,
        key: &RedisString,
        flags: KeyFlags,
    ) -> Self {
        let key_inner =
            raw::open_key_with_flags(ctx, key.inner, to_raw_mode(KeyMode::Read), flags.bits());
        Self { ctx, key_inner }
    }

    pub(crate) const fn from_raw_parts(
        ctx: *mut raw::RedisModuleCtx,
        key_inner: *mut raw::RedisModuleKey,
    ) -> Self {
        Self { ctx, key_inner }
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_ModuleTypeGetValue` is missing in redismodule.h
    pub fn get_value<T>(&self, redis_type: &RedisType) -> Result<Option<&T>, RedisError> {
        verify_type(self.key_inner, redis_type)?;

        let value =
            unsafe { raw::RedisModule_ModuleTypeGetValue.unwrap()(self.key_inner).cast::<T>() };

        if value.is_null() {
            return Ok(None);
        }

        let value = unsafe { &*value };

        Ok(Some(value))
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_KeyType` is missing in redismodule.h
    #[must_use]
    pub fn key_type(&self) -> raw::KeyType {
        let discriminant = unsafe { raw::RedisModule_KeyType.unwrap()(self.key_inner) };
        KeyType::from_repr(discriminant as u32).unwrap()
    }

    /// Detects whether the key pointer given to us by Redis is null.
    #[must_use]
    pub fn is_null(&self) -> bool {
        let null_key: *mut raw::RedisModuleKey = ptr::null_mut();
        self.key_inner == null_key
    }

    pub fn read(&self) -> Result<Option<&[u8]>, RedisError> {
        if self.is_null() {
            Ok(None)
        } else {
            let mut length: size_t = 0;
            let dma = raw::string_dma(self.key_inner, &mut length, raw::KeyMode::READ);
            if dma.is_null() {
                Err(RedisError::Str("Could not read key"))
            } else {
                Ok(Some(unsafe {
                    std::slice::from_raw_parts(dma.cast::<u8>(), length)
                }))
            }
        }
    }

    pub fn hash_get(&self, field: &str) -> Result<Option<RedisString>, RedisError> {
        let val = if self.is_null() {
            None
        } else {
            hash_mget_key(self.ctx, self.key_inner, &[field])?
                .pop()
                .expect("hash_mget_key should return vector of same length as input")
        };
        Ok(val)
    }

    /// Returns the values associated with the specified fields in the hash stored at this key.
    /// The result will be `None` if the key does not exist.
    pub fn hash_get_multi<'a, A, B>(
        &self,
        fields: &'a [A],
    ) -> Result<Option<HMGetResult<'a, A, B>>, RedisError>
    where
        A: Into<Vec<u8>> + Clone,
        RedisString: Into<B>,
    {
        let val = if self.is_null() {
            None
        } else {
            Some(HMGetResult {
                fields,
                values: hash_mget_key(self.ctx, self.key_inner, fields)?,
                phantom: std::marker::PhantomData,
            })
        };
        Ok(val)
    }

    pub fn get_stream_iterator(&self, reverse: bool) -> Result<StreamIterator<'_>, RedisError> {
        StreamIterator::new(self, None, None, false, reverse)
    }

    pub fn get_stream_range_iterator(
        &self,
        from: Option<raw::RedisModuleStreamID>,
        to: Option<raw::RedisModuleStreamID>,
        exclusive: bool,
        reverse: bool,
    ) -> Result<StreamIterator<'_>, RedisError> {
        StreamIterator::new(self, from, to, exclusive, reverse)
    }
}

impl Drop for RedisKey {
    // Frees resources appropriately as a RedisKey goes out of scope.
    fn drop(&mut self) {
        if !self.key_inner.is_null() {
            raw::close_key(self.key_inner);
        }
    }
}

/// `RedisKeyWritable` is an abstraction over a Redis key that allows read and
/// write operations.
pub struct RedisKeyWritable {
    ctx: *mut raw::RedisModuleCtx,
    key_inner: *mut raw::RedisModuleKey,
}

impl RedisKeyWritable {
    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &RedisString) -> Self {
        let key_inner = raw::open_key(ctx, key.inner, to_raw_mode(KeyMode::ReadWrite));
        Self { ctx, key_inner }
    }

    pub(crate) fn open_with_flags(
        ctx: *mut raw::RedisModuleCtx,
        key: &RedisString,
        flags: KeyFlags,
    ) -> Self {
        let key_inner = raw::open_key_with_flags(
            ctx,
            key.inner,
            to_raw_mode(KeyMode::ReadWrite),
            flags.bits(),
        );
        Self { ctx, key_inner }
    }

    /// Returns `true` if the key is of type [KeyType::Empty].
    ///
    /// # Note
    ///
    /// An empty key can be reliably detected by looking for a null
    /// as the key is opened [RedisKeyWritable::open] in read mode,
    /// but when asking for a write, Redis returns a non-null pointer
    /// to allow to write to even an empty key. In that case, the key's
    /// value should be checked manually instead:
    ///
    /// ```
    /// use redis_module::key::RedisKeyWritable;
    /// use redis_module::RedisError;
    ///
    /// fn is_empty_old(key: &RedisKeyWritable) -> Result<bool, RedisError> {
    ///     let mut s = key.as_string_dma()?;
    ///     let is_empty = s.write(b"new value")?.is_empty();
    ///     Ok(is_empty)
    /// }
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.key_type() == KeyType::Empty
    }

    pub fn as_string_dma(&self) -> Result<StringDMA<'_>, RedisError> {
        StringDMA::new(self)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn hash_set(&self, field: &str, value: RedisString) -> raw::Status {
        raw::hash_set(self.key_inner, field, value.inner)
    }

    #[allow(clippy::must_use_candidate)]
    pub fn hash_del(&self, field: &str) -> raw::Status {
        raw::hash_del(self.key_inner, field)
    }

    pub fn hash_get(&self, field: &str) -> Result<Option<RedisString>, RedisError> {
        Ok(hash_mget_key(self.ctx, self.key_inner, &[field])?
            .pop()
            .expect("hash_mget_key should return vector of same length as input"))
    }

    /// Returns the values associated with the specified fields in the hash stored at this key.
    pub fn hash_get_multi<'a, A, B>(
        &self,
        fields: &'a [A],
    ) -> Result<HMGetResult<'a, A, B>, RedisError>
    where
        A: Into<Vec<u8>> + Clone,
        RedisString: Into<B>,
    {
        Ok(HMGetResult {
            fields,
            values: hash_mget_key(self.ctx, self.key_inner, fields)?,
            phantom: std::marker::PhantomData,
        })
    }

    // `list_push_head` inserts the specified element at the head of the list stored at this key.
    #[allow(clippy::must_use_candidate)]
    pub fn list_push_head(&self, element: RedisString) -> raw::Status {
        raw::list_push(self.key_inner, raw::Where::ListHead, element.inner)
    }

    // `list_push_tail` inserts the specified element at the tail of the list stored at this key.
    #[allow(clippy::must_use_candidate)]
    pub fn list_push_tail(&self, element: RedisString) -> raw::Status {
        raw::list_push(self.key_inner, raw::Where::ListTail, element.inner)
    }

    //  `list_pop_head` pops and returns the first element of the list.
    //  Returns None when:
    //     1. The list is empty.
    //     2. The key is not a list.
    #[allow(clippy::must_use_candidate)]
    pub fn list_pop_head(&self) -> Option<RedisString> {
        let ptr = raw::list_pop(self.key_inner, raw::Where::ListHead);

        if ptr.is_null() {
            return None;
        }

        Some(RedisString::new(NonNull::new(self.ctx), ptr))
    }

    //  `list_pop_head` pops and returns the last element of the list.
    //  Returns None when:
    //     1. The list is empty.
    //     2. The key is not a list.
    #[must_use]
    pub fn list_pop_tail(&self) -> Option<RedisString> {
        let ptr = raw::list_pop(self.key_inner, raw::Where::ListTail);

        if ptr.is_null() {
            return None;
        }

        Some(RedisString::new(NonNull::new(self.ctx), ptr))
    }

    pub fn set_expire(&self, expire: Duration) -> RedisResult {
        let exp_millis = expire.as_millis();

        let exp_time = i64::try_from(exp_millis).map_err(|_| {
            RedisError::String(format!("Error expire duration {exp_millis} is not allowed"))
        })?;

        match raw::set_expire(self.key_inner, exp_time) {
            raw::Status::Ok => REDIS_OK,

            // Error may occur if the key wasn't open for writing or is an
            // empty key.
            raw::Status::Err => Err(RedisError::Str("Error while setting key expire")),
        }
    }

    /// Remove expiration from a key if it exists.
    pub fn remove_expire(&self) -> RedisResult {
        match raw::set_expire(self.key_inner, REDISMODULE_NO_EXPIRE.into()) {
            raw::Status::Ok => REDIS_OK,

            // Error may occur if the key wasn't open for writing or is an
            // empty key.
            raw::Status::Err => Err(RedisError::Str("Error while removing key expire")),
        }
    }

    pub fn write(&self, val: &str) -> RedisResult {
        let val_str = RedisString::create(NonNull::new(self.ctx), val);
        match raw::string_set(self.key_inner, val_str.inner) {
            raw::Status::Ok => REDIS_OK,
            raw::Status::Err => Err(RedisError::Str("Error while setting key")),
        }
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_DeleteKey` is missing in redismodule.h
    pub fn delete(&self) -> RedisResult {
        unsafe { raw::RedisModule_DeleteKey.unwrap()(self.key_inner) };
        REDIS_OK
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_UnlinkKey` is missing in redismodule.h
    pub fn unlink(&self) -> RedisResult {
        unsafe { raw::RedisModule_UnlinkKey.unwrap()(self.key_inner) };
        REDIS_OK
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_KeyType` is missing in redismodule.h
    #[must_use]
    pub fn key_type(&self) -> raw::KeyType {
        let discriminant = unsafe { raw::RedisModule_KeyType.unwrap()(self.key_inner) };
        KeyType::from_repr(discriminant as u32).unwrap()
    }

    pub fn open_with_redis_string(
        ctx: *mut raw::RedisModuleCtx,
        key: *mut raw::RedisModuleString,
    ) -> Self {
        let key_inner = raw::open_key(ctx, key, to_raw_mode(KeyMode::ReadWrite));
        Self { ctx, key_inner }
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_ModuleTypeGetValue` is missing in redismodule.h
    ///
    /// TODO Avoid clippy warning about needless lifetime as a temporary workaround
    #[allow(clippy::needless_lifetimes)]
    pub fn get_value<'a, 'b, T>(
        &'a self,
        redis_type: &RedisType,
    ) -> Result<Option<&'b mut T>, RedisError> {
        verify_type(self.key_inner, redis_type)?;
        let value =
            unsafe { raw::RedisModule_ModuleTypeGetValue.unwrap()(self.key_inner).cast::<T>() };

        if value.is_null() {
            return Ok(None);
        }

        let value = unsafe { &mut *value };
        Ok(Some(value))
    }

    /// # Panics
    ///
    /// Will panic if `RedisModule_ModuleTypeSetValue` is missing in redismodule.h
    pub fn set_value<T>(&self, redis_type: &RedisType, value: T) -> Result<(), RedisError> {
        verify_type(self.key_inner, redis_type)?;
        let value = Box::into_raw(Box::new(value)).cast::<c_void>();
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

    pub fn trim_stream_by_id(
        &self,
        mut id: raw::RedisModuleStreamID,
        approx: bool,
    ) -> Result<usize, RedisError> {
        let flags = if approx {
            raw::REDISMODULE_STREAM_TRIM_APPROX
        } else {
            0
        };
        let res = unsafe {
            raw::RedisModule_StreamTrimByID.unwrap()(self.key_inner, flags as i32, &mut id)
        };
        if res <= 0 {
            Err(RedisError::Str("Failed trimming the stream"))
        } else {
            Ok(res as usize)
        }
    }
}

/// Opaque type used to hold multi-get results. Use the provided methods to convert
/// the results into the desired type of Rust collection.
pub struct HMGetResult<'a, A, B>
where
    A: Into<Vec<u8>> + Clone,
    RedisString: Into<B>,
{
    fields: &'a [A],
    values: Vec<Option<RedisString>>,
    phantom: std::marker::PhantomData<B>,
}

pub struct HMGetIter<'a, A, B>
where
    A: Into<Vec<u8>>,
    RedisString: Into<B>,
{
    fields_iter: std::slice::Iter<'a, A>,
    values_iter: std::vec::IntoIter<Option<RedisString>>,
    phantom: std::marker::PhantomData<B>,
}

impl<'a, A, B> Iterator for HMGetIter<'a, A, B>
where
    A: Into<Vec<u8>> + Clone,
    RedisString: Into<B>,
{
    type Item = (A, B);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let a = self.fields_iter.next();
            let b = self.values_iter.next();
            match b {
                None => return None,
                Some(None) => continue,
                Some(Some(rs)) => {
                    return Some((
                        a.expect("field and value slices not of same length")
                            .clone(),
                        rs.into(),
                    ))
                }
            }
        }
    }
}

impl<'a, A, B> IntoIterator for HMGetResult<'a, A, B>
where
    A: Into<Vec<u8>> + Clone,
    RedisString: Into<B>,
{
    type Item = (A, B);
    type IntoIter = HMGetIter<'a, A, B>;

    /// Provides an iterator over the multi-get results in the form of (field-name, field-value)
    /// pairs. The type of field-name elements is the same as that passed to the original multi-
    /// get call, while the field-value elements may be of any type for which a `RedisString` `Into`
    /// conversion is implemented.
    ///
    /// # Examples
    ///
    /// Get a [`HashMap`] from the results:
    ///
    /// ```
    /// use redis_module::key::HMGetResult;
    /// use redis_module::{Context, RedisError, RedisResult, RedisString, RedisValue};
    ///
    /// fn call_hash(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    ///     let key_name = RedisString::create(None, "config");
    ///     let fields = &["username", "password", "email"];
    ///     let hm: HMGetResult<'_, &str, RedisString> = ctx
    ///         .open_key(&key_name)
    ///         .hash_get_multi(fields)?
    ///         .ok_or(RedisError::Str("ERR key not found"))?;
    ///     let response: Vec<RedisValue> = hm.into_iter().map(|(_, v)| v.into()).collect();
    ///     Ok(RedisValue::Array(response))
    /// }
    /// ```
    ///
    /// Get a [`Vec`] of only the field values from the results:
    ///
    /// ```
    /// use redis_module::{Context, RedisError, RedisResult, RedisString, RedisValue};
    /// use redis_module::key::HMGetResult;
    ///
    /// fn call_hash(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    ///     let key_name = RedisString::create(None, "config");
    ///     let fields = &["username", "password", "email"];
    ///     let hm: HMGetResult<'_, &str, RedisString> = ctx
    ///          .open_key(&key_name)
    ///          .hash_get_multi(fields)?
    ///          .ok_or(RedisError::Str("ERR key not found"))?;
    ///     let response: Vec<RedisValue> = hm.into_iter().map(|(_, v)| RedisValue::BulkRedisString(v)).collect();
    ///     Ok(RedisValue::Array(response))
    /// }
    /// ```
    ///
    /// [`HashMap`]: std::collections::HashMap
    /// [`Vec`]: Vec
    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            fields_iter: self.fields.iter(),
            values_iter: self.values.into_iter(),
            phantom: std::marker::PhantomData,
        }
    }
}

pub struct StringDMA<'a> {
    key: &'a RedisKeyWritable,
    buffer: &'a mut [u8],
}

impl<'a> Deref for StringDMA<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.buffer
    }
}

impl<'a> DerefMut for StringDMA<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer
    }
}

impl<'a> StringDMA<'a> {
    fn new(key: &'a RedisKeyWritable) -> Result<StringDMA<'a>, RedisError> {
        let mut length: size_t = 0;
        let dma = raw::string_dma(key.key_inner, &mut length, raw::KeyMode::WRITE);
        if dma.is_null() {
            Err(RedisError::Str("Could not read key"))
        } else {
            let buffer = unsafe { std::slice::from_raw_parts_mut(dma.cast::<u8>(), length) };
            Ok(StringDMA { key, buffer })
        }
    }

    pub fn write(&mut self, data: &[u8]) -> Result<&mut Self, RedisError> {
        if self.buffer.len() != data.len() {
            if raw::Status::Ok == raw::string_truncate(self.key.key_inner, data.len()) {
                let mut length: size_t = 0;
                let dma = raw::string_dma(self.key.key_inner, &mut length, raw::KeyMode::WRITE);
                self.buffer = unsafe { std::slice::from_raw_parts_mut(dma.cast::<u8>(), length) };
            } else {
                return Err(RedisError::Str("Failed to truncate string"));
            }
        }
        self.buffer[..data.len()].copy_from_slice(data);
        Ok(self)
    }

    pub fn append(&mut self, data: &[u8]) -> Result<&mut Self, RedisError> {
        let current_len = self.buffer.len();
        let new_len = current_len + data.len();
        if raw::Status::Ok == raw::string_truncate(self.key.key_inner, new_len) {
            let mut length: size_t = 0;
            let dma = raw::string_dma(self.key.key_inner, &mut length, raw::KeyMode::WRITE);
            self.buffer = unsafe { std::slice::from_raw_parts_mut(dma.cast::<u8>(), length) };
        } else {
            return Err(RedisError::Str("Failed to truncate string"));
        }
        self.buffer[current_len..new_len].copy_from_slice(data);
        Ok(self)
    }
}

impl Drop for RedisKeyWritable {
    // Frees resources appropriately as a RedisKey goes out of scope.
    fn drop(&mut self) {
        raw::close_key(self.key_inner);
    }
}

/// Get an arbitrary number of hash fields from a key by batching calls
/// to `raw::hash_get_multi`.
fn hash_mget_key<T>(
    ctx: *mut raw::RedisModuleCtx,
    key: *mut raw::RedisModuleKey,
    fields: &[T],
) -> Result<Vec<Option<RedisString>>, RedisError>
where
    T: Into<Vec<u8>> + Clone,
{
    const BATCH_SIZE: usize = 12;

    let mut values = Vec::with_capacity(fields.len());
    let mut values_raw = [std::ptr::null_mut(); BATCH_SIZE];

    for chunk_fields in fields.chunks(BATCH_SIZE) {
        let chunk_values = &mut values_raw[..chunk_fields.len()];
        raw::hash_get_multi(key, chunk_fields, chunk_values)?;
        values.extend(chunk_values.iter().map(|ptr| {
            if ptr.is_null() {
                None
            } else {
                Some(RedisString::from_redis_module_string(ctx, *ptr))
            }
        }));
    }

    Ok(values)
}

fn to_raw_mode(mode: KeyMode) -> raw::KeyMode {
    match mode {
        KeyMode::Read => raw::KeyMode::READ,
        KeyMode::ReadWrite => raw::KeyMode::READ | raw::KeyMode::WRITE,
    }
}

/// # Panics
///
/// Will panic if `RedisModule_KeyType` or `RedisModule_ModuleTypeGetType` are missing in redismodule.h
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn verify_type(key_inner: *mut raw::RedisModuleKey, redis_type: &RedisType) -> RedisResult {
    let key_type: KeyType = KeyType::from_repr(unsafe { raw::RedisModule_KeyType.unwrap()(key_inner) as u32 }).unwrap();

    if key_type != KeyType::Empty {
        // The key exists; check its type
        let raw_type = unsafe { raw::RedisModule_ModuleTypeGetType.unwrap()(key_inner) };

        if raw_type != *redis_type.raw_type.borrow() {
            return Err(RedisError::Str("Existing key has wrong Redis type"));
        }
    }

    REDIS_OK
}
