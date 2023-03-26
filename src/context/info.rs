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
    pub fn field(&self, field: &str) -> Option<RedisString> {
        let field = CString::new(field).unwrap();
        let value = unsafe {
            raw::RedisModule_ServerInfoGetField.unwrap()(self.ctx, self.inner, field.as_ptr())
        };
        if value.is_null() {
            None
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
