use std::convert::TryFrom;
use std::os::raw::c_void;
use std::ptr;
use std::str::Utf8Error;
use std::time::Duration;

use libc::size_t;

use raw::KeyType;

use crate::from_byte_string;
use crate::native_types::RedisType;
use crate::raw;
use crate::RedisError;
use crate::redismodule::REDIS_OK;
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
}

impl RedisKey {
    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &str) -> RedisKey {
        let key_str = RedisString::create(ctx, key);
        let key_inner = raw::open_key(ctx, key_str.inner, to_raw_mode(KeyMode::Read));
        RedisKey {
            ctx: ctx,
            key_inner: key_inner,
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

    pub fn key_type(&self) -> raw::KeyType {
        unsafe { raw::RedisModule_KeyType.unwrap()(self.key_inner) }.into()
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
}

impl RedisKeyWritable {
    pub fn open(ctx: *mut raw::RedisModuleCtx, key: &str) -> RedisKeyWritable {
        let key_str = RedisString::create(ctx, key);
        let key_inner = raw::open_key(ctx, key_str.inner, to_raw_mode(KeyMode::ReadWrite));
        RedisKeyWritable {
            ctx: ctx,
            key_inner: key_inner,
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
    pub fn list_push_head(&self, element: RedisString) -> raw::Status {
        raw::list_push(self.key_inner, raw::Where::ListHead, element.inner)
    }

    // `list_push_tail` inserts the specified element at the tail of the list stored at this key.
    pub fn list_push_tail(&self, element: RedisString) -> raw::Status {
        raw::list_push(self.key_inner, raw::Where::ListTail, element.inner)
    }

    //  `list_pop_head` pops and returns the first element of the list.
    //  Returns None when:
    //     1. The list is empty.
    //     2. The key is not a list.
    pub fn list_pop_head(&self) -> Option<RedisString> {
        let ptr = raw::list_pop(self.key_inner, raw::Where::ListHead);

        if ptr.is_null() {
            return None;
        }

        Some(RedisString::new(self.ctx, ptr))
    }

    //  `list_pop_head` pops and returns the last element of the list.
    //  Returns None when:
    //     1. The list is empty.
    //     2. The key is not a list.
    pub fn list_pop_tail(&self) -> Option<RedisString> {
        let ptr = raw::list_pop(self.key_inner, raw::Where::ListTail);

        if ptr.is_null() {
            return None;
        }

        Some(RedisString::new(self.ctx, ptr))
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

    pub fn key_type(&self) -> raw::KeyType {
        unsafe { raw::RedisModule_KeyType.unwrap()(self.key_inner) }.into()
    }

    pub fn is_empty(&self) -> bool {
        self.key_type() == KeyType::Empty
    }

    pub fn open_with_redis_string(
        ctx: *mut raw::RedisModuleCtx,
        key: *mut raw::RedisModuleString,
    ) -> RedisKeyWritable {
        let key_inner = raw::open_key(ctx, key, to_raw_mode(KeyMode::ReadWrite));
        RedisKeyWritable {
            ctx: ctx,
            key_inner: key_inner,
        }
    }

    pub fn get_value<'a, 'b, T>(&'a self, redis_type: &RedisType) -> Result<Option<&'b mut T>, RedisError> {
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
    /// get call, while the field-value elements may be of any type for which a RedisString `Into`
    /// conversion is implemented.  
    ///
    /// # Examples
    ///
    /// Get a [`HashMap`] from the results:
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use redis_module::RedisError;
    ///
    /// let keyname = "config";
    /// let fields = &["username", "password", "email"];
    /// let hm = ctx
    ///      .open_key(keyname)
    ///      .hash_get_multi(fields)?
    ///      .ok_or(RedisError::Str("ERR key not found"))?;
    /// let response: HashMap<&str, String> = hm.into_iter().collect();
    /// ```
    ///
    /// Get a [`Vec`] of only the field values from the results:
    ///
    /// ```
    /// use redis_module::RedisError;
    ///
    /// let hm = ctx
    ///      .open_key(keyname)
    ///      .hash_get_multi(fields)?
    ///      .ok_or(RedisError::Str("ERR key not found"))?;
    /// let response: Vec<String> = hm.into_iter().map(|(_, v)| v).collect();
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
        let mut chunk_values = &mut values_raw[..chunk_fields.len()];
        raw::hash_get_multi(key, chunk_fields, &mut chunk_values)?;
        values.extend(chunk_values.iter().map(|ptr| {
            if ptr.is_null() {
                None
            } else {
                Some(RedisString::new(ctx, *ptr))
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

fn verify_type(key_inner: *mut raw::RedisModuleKey, redis_type: &RedisType) -> RedisResult {
    let key_type: KeyType = unsafe { raw::RedisModule_KeyType.unwrap()(key_inner) }.into();

    if key_type != KeyType::Empty {
        // The key exists; check its type
        let raw_type = unsafe { raw::RedisModule_ModuleTypeGetType.unwrap()(key_inner) };

        if raw_type != *redis_type.raw_type.borrow() {
            return Err(RedisError::Str("Existing key has wrong Redis type"));
        }
    }

    REDIS_OK
}
