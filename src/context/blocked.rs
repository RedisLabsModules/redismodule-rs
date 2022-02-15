use std::ptr;

use crate::raw;
use crate::Context;

pub struct BlockedClient {
    pub(crate) inner: *mut raw::RedisModuleBlockedClient,
}

// We need to be able to send the inner pointer to another thread
unsafe impl Send for BlockedClient {}

impl Drop for BlockedClient {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_UnblockClient.unwrap()(self.inner, ptr::null_mut()) };
    }
}

impl Context {
    #[must_use]
    pub fn block_client(&self) -> BlockedClient {
        let blocked_client = unsafe {
            raw::RedisModule_BlockClient.unwrap()(
                self.ctx, // ctx
                None,     // reply_func
                None,     // timeout_func
                None, 0,
            )
        };

        BlockedClient {
            inner: blocked_client,
        }
    }
}
