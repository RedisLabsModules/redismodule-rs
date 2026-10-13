use std::ffi::CString;
use std::ptr::NonNull;

use crate::Context;
use crate::{raw, RedisString};

pub struct ServerInfo {
    ctx: *mut raw::RedisModuleCtx,
    pub(crate) inner: *mut raw::RedisModuleServerInfoData,
}

impl Drop for ServerInfo {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_FreeServerInfo.unwrap()(self.ctx, self.inner) };
    }
}

impl ServerInfo {
    /// Returns the value of `field` from this server-info snapshot.
    ///
    /// `RedisModule_ServerInfoGetField` hands the caller an owned string
    /// (refcount 1). With a real module context Redis tracks that string for
    /// auto-memory, and [`RedisString::new`] detaches it from the queue so
    /// Drop is the sole free. With a null context nothing tracks the string,
    /// so adopting it through [`RedisString::new`] would retain once and Drop
    /// would free once, leaving the last reference held forever. Adopt without
    /// retaining when the context is null.
    pub fn field(&self, field: &str) -> Option<RedisString> {
        let field = CString::new(field).unwrap();
        let value = unsafe {
            raw::RedisModule_ServerInfoGetField.unwrap()(self.ctx, self.inner, field.as_ptr())
        };
        if value.is_null() {
            None
        } else if self.ctx.is_null() {
            Some(RedisString::from_redis_module_string(self.ctx, value))
        } else {
            Some(RedisString::new(NonNull::new(self.ctx), value))
        }
    }
}

impl Context {
    #[must_use]
    pub fn server_info(&self, section: &str) -> ServerInfo {
        let section = CString::new(section).unwrap();
        let server_info = unsafe {
            raw::RedisModule_GetServerInfo.unwrap()(
                self.ctx,         // ctx
                section.as_ptr(), // section
            )
        };

        ServerInfo {
            ctx: self.ctx,
            inner: server_info,
        }
    }
}
