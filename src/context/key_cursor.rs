use std::{
    ffi::c_void,
    ptr::{self},
};

use crate::{key::RedisKey, raw, RedisString};

/// A cursor to scan field/value pairs of a (hash) key.
///
/// It provides access via a closure given to [`ScanKeyCursor::for_each`] or if you need more control, you can use [`ScanKeyCursor::scan`]
/// and implement your own loop, e.g. to allow an early stop.
///
/// ## Example usage
///
/// Here we show how to extract values to communicate them back to the Redis client. We assume that the following hash key is setup before:
///
/// ```text
/// HSET user:123 name Alice age 29 location Austin
/// ```
///
/// The following example command implementation scans all fields and values in the hash key and returns them as an array of RedisString.
///
/// ```ignore
/// fn example_scan_key_for_each(ctx: &Context) -> RedisResult {
///    let key = ctx.open_key_with_flags("user:123", KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
///    let cursor  = ScanKeyCursor::new(key);
///    
///    let res = RefCell::new(Vec::new());
///    cursor.for_each(|_key, field, value| {
///        let mut res = res.borrow_mut();
///        res.push(RedisValue::BulkRedisString(field.clone()));
///        res.push(RedisValue::BulkRedisString(value.clone()));
///    });
///
///    Ok(RedisValue::Array(res.take()))
/// }
/// ```
///
/// The method will produce the following output:
///
/// ```text
/// 1) "name"
/// 2) "Alice"
/// 3) "age"
/// 4) "29"
/// 5) "location"
/// 6) "Austin"
/// ```
pub struct ScanKeyCursor {
    key: RedisKey,
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

impl ScanKeyCursor {
    /// Creates a new scan cursor for the given key.
    pub fn new(key: RedisKey) -> Self {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        Self { key, inner_cursor }
    }

    /// Restarts the cursor from the beginning.
    pub fn restart(&self) {
        unsafe { raw::RedisModule_ScanCursorRestart.unwrap()(self.inner_cursor) };
    }

    pub fn scan<F: FnMut(&RedisKey, &RedisString, &RedisString)>(&self, f: F) -> bool {
        // The following is the callback definition. The callback may be called multiple times per `RedisModule_ScanKey` invocation.
        // The callback is used by [`ScanKeyCursor::scan`] and [`ScanKeyCursor::for_each`] as argument to `RedisModule_ScanKey`.
        //
        // The `data` pointer is the closure given to [`ScanKeyCursor::scan`] or [`ScanKeyCursor::for_each`]. 
        // The callback forwards references to the key, field and value to that closure.
        unsafe extern "C" fn scan_callback<
            F: FnMut(&RedisKey, &RedisString, &RedisString),
        >(
            key: *mut raw::RedisModuleKey,
            field: *mut raw::RedisModuleString,
            value: *mut raw::RedisModuleString,
            data: *mut c_void,
        ) {
            let ctx = ptr::null_mut();
            let key = RedisKey::from_raw_parts(ctx, key);

            let field = RedisString::from_redis_module_string(ctx, field);
            let value = RedisString::from_redis_module_string(ctx, value);

            let callback = unsafe { &mut *(data.cast::<F>()) };
            callback(&key, &field, &value);

            // we're not the owner of field and value strings
            field.take();
            value.take();

            key.take(); // we're not the owner of the key either
        }

        // Safety: The c-side initialized the function ptr and it is is never changed,
        // i.e. after module initialization the function pointers stay valid till the end of the program.
        let res = unsafe {
            raw::RedisModule_ScanKey.unwrap()(
                self.key.key_inner,
                self.inner_cursor,
                Some(scan_callback::<F>),
                &f as *const F as *mut c_void,
            )
        };

        res != 0
    }

    /// Implements a callback based for_each loop over all fields and values in the hash key, use that for optimal performance.
    pub fn for_each<F: FnMut(&RedisKey, &RedisString, &RedisString)>(&self, mut f: F) {
        while self.scan(&mut f) {
            // do nothing, the callback does the work
        }
    }
}

impl Drop for ScanKeyCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
