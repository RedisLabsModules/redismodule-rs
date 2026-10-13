use std::ffi::CString;
use std::ptr;

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
    /// Returns [`None`] if the field is missing or its name contains an embedded NUL byte.
    pub fn field(&self, field: &str) -> Option<RedisString> {
        let field = CString::new(field).ok()?;
        // Request an owned string outside auto-memory, regardless of the context's settings.
        // SAFETY: self.inner is the live server-info handle and field is a valid C string.
        let value = unsafe {
            raw::RedisModule_ServerInfoGetField.unwrap()(
                ptr::null_mut(),
                self.inner,
                field.as_ptr(),
            )
        };
        if value.is_null() {
            None
        } else {
            // Wrap the returned string without incrementing its reference count.
            // Keep the original context in the wrapper.
            Some(RedisString::from_redis_module_string(self.ctx, value))
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
