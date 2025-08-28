use std::{
    ffi::c_void,
    ptr::{self, addr_of_mut},
};

use crate::{key::RedisKey, raw, Context, RedisString};

/// A cursor to scan field/value pairs of a (hash) key.
///
/// This is a wrapper around the [RedisModule_ScanKey](https://redis.io/docs/latest/develop/reference/modules/modules-api-ref/#redismodule_scankey) 
/// function from the C API. It provides access via a closure given to [`ScanKeyCursor::foreach`] and alternatively 
/// provides a Rust iterator via [`ScanKeyCursor::iter`].
/// 
/// Use `foreach` if the operation requires no copies and high performance. Use the iterator variant if you need to collect the results and/or
/// want to have access to the Rust iterator API.
///
/// ## Example usage
/// 
/// Here we show how to extract values to communicate them back to the Redis client. We assume that the following hash key is setup:
/// 
/// ```text
/// HSET user:123 name Alice age 29 location Austin
/// ```
/// 
/// For using the `foreach` method:
/// 
/// ```ignore
/// fn example_scan_key_foreach(ctx: &Context) -> RedisResult {
///    let key = ctx.open_key_with_flags("user:123", KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
///    let cursor  = ScanKeyCursor::new(key);
///    
///    let res = RefCell::new(Vec::new());
///    cursor.foreach(|_key, field, value| {
///        let mut res = res.borrow_mut();
///        res.push(RedisValue::BulkRedisString(field.clone()));
///        res.push(RedisValue::BulkRedisString(value.clone()));
///    });
///
///    Ok(RedisValue::Array(res.take()))
/// }
/// ```
/// 
/// For using the `iter` method:
/// 
/// ```ignore
/// fn example_scan_key_foreach(ctx: &Context) -> RedisResult {
///     let mut res = Vec::new();
///     let key = ctx.open_key_with_flags("user:123", KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
///     let cursor  = ScanKeyCursor::new(key);
///     for (field, value) in cursor.iter().enumerate() {
///         res.push(RedisValue::BulkRedisString(field));
///         res.push(RedisValue::BulkRedisString(value));
///     }
///     Ok(RedisValue::Array(res))
/// }
/// ```
/// 
/// Both methods will produce the following output:
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

    /// Implements a callback based foreach loop over all fields and values in the hash key, use that for optimal performance.
    pub fn foreach<F: Fn(&RedisKey, &RedisString, &RedisString)>(&self, f: F) {
        // Safety: Assumption: c-side initialized the function ptr and it is is never changed,
        // i.e. after module initialization the function pointers stay valid till the end of the program.
        let scan_key = unsafe { raw::RedisModule_ScanKey.unwrap() };

        let mut res = 1;
        while res != 0 {
            res = unsafe {
                scan_key(
                    self.key.key_inner,
                    self.inner_cursor,
                    Some(foreach_callback::<F>),
                    &f as *const F as *mut c_void,
                )
            }
        }
    }

    /// Returns an iterator over all fields and values in the hash key. 
    /// 
    /// As the rust loop scope is detached from the C callback
    /// we need to buffer the field/value pairs. That has performance implications. They are lower if the field/value pairs are
    /// copied anyway, but even in that case not as fast as using the [`ScanKeyCursor::foreach`] method.
    pub fn iter(&self) -> ScanKeyCursorIterator<'_> {
        ScanKeyCursorIterator {
            cursor: self,
            buf: Vec::new(),
            last_call: false,
        }
    }
}

impl Drop for ScanKeyCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}

pub type ScanKeyIteratorItem = (RedisString, RedisString);

/// An iterator (state) over the field/value pairs of a hash key.
pub struct ScanKeyCursorIterator<'a> {
    /// The cursor that is used for the iteration
    cursor: &'a ScanKeyCursor,

    // todo: use a vector with stack allocation for better performance
    /// Buffer to hold the uninitialized data if the C callback is called multiple times.
    buf: Vec<ScanKeyIteratorItem>,

    /// Stores a flag that indicates if scan_key needs to be called again
    last_call: bool,
}

/// The state machine for the iterator
enum IteratorState {
    NeedToCallScanKey,
    HasBufferedItems,
    Done,
}

/// A stack slot that is used to pass data from the C callback to the iterator.
/// 
/// It is mainly used to access the context and to store the buffered items.
struct StackSlot<'a> {
    ctx: Context,
    buf: &'a mut Vec<ScanKeyIteratorItem>,
}

impl ScanKeyCursorIterator<'_> {
    fn current_state(&self) -> IteratorState {
        if !self.buf.is_empty() {
            IteratorState::HasBufferedItems
        } else if self.last_call {
            IteratorState::Done
        } else {
            IteratorState::NeedToCallScanKey
        }
    }

    fn next_scan_call(&mut self) -> Option<ScanKeyIteratorItem> {
        let ctx_ptr = self.cursor.key.ctx;

        let mut stack_slot = StackSlot {
            ctx: Context::new(ctx_ptr),
            buf: &mut self.buf,
        };

        let data_ptr = addr_of_mut!(stack_slot).cast::<c_void>();

        // Safety: Assumption: c-side initialized the function ptr and it is is never changed,
        // i.e. after module initialization the function pointers stay valid till the end of the program.
        let scan_key = unsafe { raw::RedisModule_ScanKey.unwrap() };

        // Safety: All pointers we pass here are guaranteed to remain valid during the `scan_key` call.
        let ret = unsafe {
            scan_key(
                self.cursor.key.key_inner,
                self.cursor.inner_cursor,
                Some(iterator_callback),
                data_ptr,
            )
        };

        // Check if more calls are needed
        if ret == 0 {
            self.last_call = true;
            // we may still have buffered items
        }

        if stack_slot.buf.is_empty() {
            // no items were returned, try again
            None
        } else {
            self.next_buffered_item()
        }
    }

    fn next_buffered_item(&mut self) -> Option<ScanKeyIteratorItem> {
        // todo: use different datatype / access pattern for performance
        Some(self.buf.remove(0))
    }
}

/// The callback that is used by [`ScanKeyCursor::foreach`] as argument to `RedisModule_ScanKey`.
///
/// The `data` pointer is the closure given to [`ScanKeyCursor::foreach`] and the callback forwards
/// references to the key, field and value to that closure.
unsafe extern "C" fn foreach_callback<F: Fn(&RedisKey, &RedisString, &RedisString)>(
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

/// The callback that is used inside the iterator variant accessible via [`ScanKeyCursor::iter`] for `RedisModule_ScanKey`. 
/// 
/// It buffers copies of the field and value strings as the lifetime of them ends when with the call
/// to `RedisModule_ScanKey` going out of scope.
/// 
/// The `data` pointer is a stack slot of type [StackSlot] that is used to pass the data back to the iterator.
unsafe extern "C" fn iterator_callback(
    _key: *mut raw::RedisModuleKey,
    field: *mut raw::RedisModuleString,
    value: *mut raw::RedisModuleString,
    data: *mut c_void,
) {
    // `data` is a stack slot
    let slot = data.cast::<StackSlot>();
    let slot = &mut (*slot);

    // todo: use new-type with refcount handling for better performance, otherwise we have to copy at this point
    // we know that this callback will be called in a loop scope and that we 
    // need to store the RedisString longer than the ScanKey invocation but not much
    // longer and in case of batched results we don't need to store everything in memory
    // but only the last batch.
    let field = raw::RedisModule_CreateStringFromString.unwrap()(slot.ctx.get_raw(), field);
    let value = raw::RedisModule_CreateStringFromString.unwrap()(slot.ctx.get_raw(), value);

    let field = RedisString::from_redis_module_string(slot.ctx.get_raw(), field);
    let value = RedisString::from_redis_module_string(slot.ctx.get_raw(), value);

    slot.buf.push((field, value));
}

// Implements an iterator for `KeysCursor` that yields (RedisString, RedisString) in a Rust for loop.
// This is a wrapper around the RedisModule_ScanKey function from the C API and uses a pattern to get the values from the callback that
// is also used in stack unwinding scenarios. There is not common term for that but here we can think of it as a "stack slot" pattern.
impl Iterator for ScanKeyCursorIterator<'_> {
    type Item = ScanKeyIteratorItem;

    fn next(&mut self) -> Option<Self::Item> {
        let ctx = Context::new(self.cursor.key.ctx);
        ctx.log_notice("ScanKeyCursorIterator next() called");
        match self.current_state() {
            IteratorState::NeedToCallScanKey => self.next_scan_call(),
            IteratorState::HasBufferedItems => self.next_buffered_item(),
            IteratorState::Done => None,
        }
    }
}
