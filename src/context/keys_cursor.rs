use crate::context::Context;
use crate::key::RedisKey;
use crate::raw;
use crate::redismodule::RedisString;
use std::ffi::c_void;
use std::mem;
use std::ptr::NonNull;

pub struct KeysCursor {
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

extern "C" fn scan_callback<C: FnMut(&Context, &RedisString, Option<&RedisKey>)>(
    ctx: *mut raw::RedisModuleCtx,
    key_name: *mut raw::RedisModuleString,
    key: *mut raw::RedisModuleKey,
    private_data: *mut ::std::os::raw::c_void,
) {
    let context = Context::new(ctx);
    let key_name = RedisString::new(NonNull::new(ctx), key_name);
    let redis_key = if key.is_null() {
        None
    } else {
        // Safety: The returned `RedisKey` does not outlive this callbacks and so by necessity
        // the pointers passed in as parameters are valid for its entire lifetime.
        Some(unsafe { RedisKey::from_raw_parts(ctx, key) })
    };
    let callback = unsafe { &mut *(private_data.cast::<C>()) };
    callback(&context, &key_name, redis_key.as_ref());

    // We don't own any of the passed in pointers and have just created "temporary RAII types".
    // We must ensure we don't run their destructors here.
    mem::forget(redis_key);
    mem::forget(key_name);
}

impl KeysCursor {
    pub fn new() -> Self {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        Self { inner_cursor }
    }

    pub fn scan<F: FnMut(&Context, &RedisString, Option<&RedisKey>)>(
        &self,
        ctx: &Context,
        callback: &F,
    ) -> bool {
        let res = unsafe {
            raw::RedisModule_Scan.unwrap()(
                ctx.ctx,
                self.inner_cursor,
                Some(scan_callback::<F>),
                callback as *const F as *mut c_void,
            )
        };
        res != 0
    }

    pub fn restart(&self) {
        unsafe { raw::RedisModule_ScanCursorRestart.unwrap()(self.inner_cursor) };
    }
}

impl Default for KeysCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeysCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
