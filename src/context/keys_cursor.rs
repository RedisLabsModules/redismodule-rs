use crate::context::Context;
use crate::key::RedisKey;
use crate::raw;
use crate::RedisString;
use std::os::raw::c_void;

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
    let mut redis_key = if !key.is_null() {
        Some(RedisKey {
            ctx: ctx,
            key_inner: key,
        })
    } else {
        None
    };
    let callback = unsafe { &mut *(privdata as *mut C) };
    callback(&context, key_name, redis_key.as_ref());

    if redis_key.is_some() {
        // we are not the owner of the key so we must not keep it.
        redis_key.as_mut().unwrap().key_inner = std::ptr::null_mut();
    }
}

impl KeysCursor {
    pub fn new() -> KeysCursor {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        KeysCursor { inner_cursor }
    }

    pub fn scan<C: FnMut(&Context, RedisString, Option<&RedisKey>)>(
        &self,
        ctx: &Context,
        callback: &C,
    ) -> bool {
        let res = unsafe {
            raw::RedisModule_Scan.unwrap()(
                ctx.ctx,
                self.inner_cursor,
                Some(scan_callback::<C>),
                callback as *const C as *mut c_void,
            )
        };
        if res != 0 {
            true
        } else {
            false
        }
    }
}

impl Drop for KeysCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
