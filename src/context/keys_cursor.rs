use crate::context::Context;
use crate::key::RedisKey;
use crate::raw;
use crate::redismodule::RedisString;
use std::ffi::c_void;

pub struct KeysCursor {
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

extern "C" fn scan_callback<C: FnMut(&Context, RedisString, Option<&RedisKey>)>(
    ctx: *mut raw::RedisModuleCtx,
    keyname: *mut raw::RedisModuleString,
    key: *mut raw::RedisModuleKey,
    privdata: *mut ::std::os::raw::c_void,
) {
    let context = Context::new(ctx);
    let key_name = RedisString::new(ctx, keyname);
    let redis_key = if !key.is_null() {
        Some(RedisKey::from_raw_parts(ctx, key))
    } else {
        None
    };
    let callback = unsafe { &mut *(privdata as *mut C) };
    callback(&context, key_name, redis_key.as_ref());

    // we are not the owner of the key, so we must take the underline *mut raw::RedisModuleKey so it will not be freed.
    redis_key.map(|v| v.take());
}

impl KeysCursor {
    pub fn new() -> KeysCursor {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        KeysCursor { inner_cursor }
    }

    pub fn scan<F: FnMut(&Context, RedisString, Option<&RedisKey>)>(
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

impl Drop for KeysCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
