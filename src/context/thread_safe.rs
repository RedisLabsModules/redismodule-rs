use std::ops::Deref;
use std::ptr;

use crate::{raw, Context};

pub struct ContextGuard {
    ctx: Context,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ThreadSafeContextUnlock.unwrap()(self.ctx.ctx) };
    }
}

impl Deref for ContextGuard {
    type Target = Context;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

pub struct ThreadSafeContext {
    pub(crate) ctx: *mut raw::RedisModuleCtx,
}

impl ThreadSafeContext {
    pub fn new() -> ThreadSafeContext {
        let ctx = unsafe { raw::RedisModule_GetThreadSafeContext.unwrap()(ptr::null_mut()) };
        ThreadSafeContext { ctx }
    }

    pub fn lock(&self) -> ContextGuard {
        unsafe { raw::RedisModule_ThreadSafeContextLock.unwrap()(self.ctx) };
        let ctx = Context::new(self.ctx);
        ContextGuard { ctx }
    }
}

impl Drop for ThreadSafeContext {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_FreeThreadSafeContext.unwrap()(self.ctx) };
    }
}
