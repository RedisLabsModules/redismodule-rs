use std::os::raw::{c_char, c_int};

use crate::raw::{self, RedisModuleString, REDISMODULE_NOTIFY_ALL, REDISMODULE_NOTIFY_GENERIC};
use crate::Context;

impl Context {
    /// Wrapper for `RedisModule_SubscribeToKeyspaceEvents`.
    /// A note about this redis module api:
    /// Executing the callback is going to block Redis. You'll probably want to
    /// move processing these events to a channel and thread pool.
    pub fn subscribe_keyspace_events(
        &self,
        types: isize,
        cb: unsafe extern "C" fn(
            *mut raw::RedisModuleCtx,
            c_int,
            *const c_char,
            *mut RedisModuleString,
        ) -> i32,
    ) -> i32 {
        // types value should be within this range
        if types < REDISMODULE_NOTIFY_GENERIC as isize || types > REDISMODULE_NOTIFY_ALL as isize {
            return 1;
        }
        unsafe {
            raw::RedisModule_SubscribeToKeyspaceEvents.unwrap()(self.ctx, types as i32, Some(cb))
        }
    }
}
