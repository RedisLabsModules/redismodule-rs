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
        // Use null for allocation and cleanup so the string can outlive its command context.
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
            Some(RedisString::from_redis_module_string(
                ptr::null_mut(),
                value,
            ))
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
