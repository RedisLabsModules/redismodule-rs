use std::ops::Deref;
use std::ptr;

use crate::context::blocked::BlockedClient;
use crate::{raw, Context, RedisResult};

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

/// A ThreadSafeContext can either be bound to a blocked client, or detached from any client.
pub struct DetachedFromClient;

pub struct ThreadSafeContext<B> {
    pub(crate) ctx: *mut raw::RedisModuleCtx,

    /// This field is only used implicitly by `Drop`, so avoid a compiler warning
    #[allow(dead_code)]
    blocked_client: B,
}

unsafe impl<B> Send for ThreadSafeContext<B> {}
unsafe impl<B> Sync for ThreadSafeContext<B> {}

impl ThreadSafeContext<DetachedFromClient> {
    pub fn new() -> Self {
        let ctx = unsafe { raw::RedisModule_GetThreadSafeContext.unwrap()(ptr::null_mut()) };
        ThreadSafeContext {
            ctx,
            blocked_client: DetachedFromClient,
        }
    }
}

impl ThreadSafeContext<BlockedClient> {
    pub fn with_blocked_client(blocked_client: BlockedClient) -> Self {
        let ctx = unsafe { raw::RedisModule_GetThreadSafeContext.unwrap()(blocked_client.inner) };
        ThreadSafeContext {
            ctx,
            blocked_client,
        }
    }

    /// The Redis modules API does not require locking for `Reply` functions,
    /// so we pass through its functionality directly.
    pub fn reply(&self, r: RedisResult) -> raw::Status {
        let ctx = Context::new(self.ctx);
        ctx.reply(r)
    }
}

impl<B> ThreadSafeContext<B> {
    /// All other APIs require locking the context, so we wrap it in a way
    /// similar to `std::sync::Mutex`.
    pub fn lock(&self) -> ContextGuard {
        unsafe { raw::RedisModule_ThreadSafeContextLock.unwrap()(self.ctx) };
        let ctx = Context::new(self.ctx);
        ContextGuard { ctx }
    }
}

impl<B> Drop for ThreadSafeContext<B> {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_FreeThreadSafeContext.unwrap()(self.ctx) };
    }
}
