use std::{ffi::c_void, ptr::{self, addr_of_mut}};

use crate::{key::RedisKey, raw, Context, RedisString};

/// A cursor to scan fields and values in a hash key.
/// 
/// This is a wrapper around the RedisModule_ScanKey function from the C API. It provides access via [`ScanKeyCursor::foreach] and provides
/// a Rust iterator.
/// 
/// Example usage:
/// ```no_run
/// 
/// ```
/// 
/// The iterator yields tuples of (field: RedisString, value: RedisString).
/// 
/// ## Implementation notes
/// 
/// The `RedisModule_ScanKey` function from the C API uses a callback to return the field and value strings. We
/// distinguish two cases:
/// 
/// 1. Either the callback is called once, 
/// 2. or multiple times 
/// 
/// and this depends if a rehash happens during the scan.
pub struct ScanKeyCursor {
    key: RedisKey,  
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

//type ScanKeyCallback<F> = F where F: FnMut(&RedisKey, &RedisString, &RedisString);

impl ScanKeyCursor {
    pub fn new(key: RedisKey) -> Self {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        Self { key, inner_cursor }
    }

    pub fn restart(&self) {
        unsafe { raw::RedisModule_ScanCursorRestart.unwrap()(self.inner_cursor) };
    }

    /// Implements a callback based foreach loop over all fields and values in the hash key, use for optimal performance.
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

    pub fn iter(&self) -> ScanKeyCursorIterator<'_> {
        let ctx = Context::new(self.key.ctx);
        ctx.log_notice("Starting ScanKeyCursor iteration");
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

pub struct ScanKeyCursorIterator<'a> {
    /// The cursor that is used for the iteration
    cursor: &'a ScanKeyCursor,

    /// Buffer to hold the uninitialized data if the C callback is called multiple times.
    buf: Vec<ScanKeyIteratorItem>,

    /// Stores a flag that indicates if scan_key needs to be called again
    last_call: bool,
}

enum IteratorState {
    NeedToCallScanKey,
    HasBufferedItems,
    Done,
}

enum StackSlotState {
    Empty,
    Filled(ScanKeyIteratorItem),
}

struct StackSlot<'a> {
    state: StackSlotState,
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
        let ctx = Context::new(ctx_ptr);
        
        let mut stack_slot = StackSlot {
            state: StackSlotState::Empty,
            ctx: Context::new(ctx_ptr),
            buf: &mut self.buf
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

        let StackSlotState::Filled(reval) = stack_slot.state else {
            // should not happen
            panic!("ScanKey callback did not fill the stack slot");
        };

        ctx.log_notice(&format!("next Reval field: {}, value: {}", reval.0, reval.1));

        Some(reval)
    }

    fn next_buffered_item(&mut self) -> Option<ScanKeyIteratorItem> {
        // todo: use different datatype / access pattern for performance
        Some(self.buf.remove(0))
    }
}

/// The callback that is called by `RedisModule_ScanKey` to return the field and value strings.
/// 
/// The `data` pointer is a stack slot of type `RawData` that is used to pass the data back to the iterator.
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

/// The callback that is called by `RedisModule_ScanKey` to return the field and value strings.
/// 
/// The `data` pointer is a stack slot of type `RawData` that is used to pass the data back to the iterator.
unsafe extern "C" fn iterator_callback(
    _key: *mut raw::RedisModuleKey,
    field: *mut raw::RedisModuleString,
    value: *mut raw::RedisModuleString,
    data: *mut c_void,
) {
    // SAFETY: this is the responsibility of the caller, see only usage below in `next()`
    // `data` is a stack slot of type Data
    let slot = data.cast::<StackSlot>();
    let slot = &mut (*slot);

    // todo: use new-type with refcount handling for better performance
    let field = raw::RedisModule_CreateStringFromString.unwrap()(slot.ctx.get_raw(), field);
    let value = raw::RedisModule_CreateStringFromString.unwrap()(slot.ctx.get_raw(), value);

    let field = RedisString::from_redis_module_string(slot.ctx.get_raw(), field);
    let value = RedisString::from_redis_module_string(slot.ctx.get_raw(), value);

    match slot.state {
        StackSlotState::Empty => {
            let out = format!("CB - Fill empty slot - Field: {}, Value: {}", field, value);
            slot.ctx.log_notice(&out);
            slot.state = StackSlotState::Filled((field, value));
        }
        StackSlotState::Filled(_) => {
            // This is the case where the callback is called multiple times.
            // We need to buffer the data in the iterator state.
            let out = format!("CB - Buffer for future use - Field: {}, Value: {}", field, value);
            slot.ctx.log_notice(&out);
            slot.buf.push((field, value));
            
        }
    }

}

// Implements an iterator for `KeysCursor` that yields (RedisKey, *mut RedisModuleString, *mut RedisModuleString) in a Rust for loop.
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