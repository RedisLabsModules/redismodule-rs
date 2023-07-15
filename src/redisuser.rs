use std::{ffi::CString, os::raw::c_char};

use crate::{raw, AclPermissions, RedisError, RedisString};

pub struct RedisUser {
    pub(super) user: *mut raw::RedisModuleUser,
}

impl RedisUser {
    pub fn new(username: &str) -> RedisUser {
        let username = CString::new(username).unwrap();
        let module_user = unsafe { raw::RedisModule_CreateModuleUser.unwrap()(username.as_ptr()) };

        RedisUser { user: module_user }
    }

    pub(super) fn from_redis_module_user(user: *mut raw::RedisModuleUser) -> RedisUser {
        RedisUser { user }
    }

    pub fn set_acl(&self, acl: &str) -> Result<(), RedisError> {
        let acl = CString::new(acl).unwrap();
        let mut error: *mut raw::RedisModuleString = std::ptr::null_mut();
        let error_ptr: *mut *mut raw::RedisModuleString = &mut error;

        let result = unsafe {
            raw::RedisModule_SetModuleUserACLString.unwrap()(
                std::ptr::null_mut(),
                self.user,
                acl.as_ptr().cast::<c_char>(),
                error_ptr,
            )
        };

        // If the result is an error, parse the error string
        if result != raw::REDISMODULE_OK as i32 {
            let error = RedisString::from_redis_module_string(std::ptr::null_mut(), error);
            return Err(RedisError::String(error.to_string_lossy()));
        }

        Ok(())
    }

    pub fn acl(&self) -> RedisString {
        let acl = unsafe { raw::RedisModule_GetModuleUserACLString.unwrap()(self.user) };
        RedisString::from_redis_module_string(std::ptr::null_mut(), acl)
    }

    /// Verify the the given user has the give ACL permission on the given key.
    /// Return Ok(()) if the user has the permissions or error (with relevant error message)
    /// if the validation failed.
    pub fn acl_check_key_permission(
        &self,
        key_name: &RedisString,
        permissions: &AclPermissions,
    ) -> Result<(), RedisError> {
        let acl_permission_result: raw::Status = unsafe {
            raw::RedisModule_ACLCheckKeyPermissions.unwrap()(
                self.user,
                key_name.inner,
                permissions.bits(),
            )
        }
        .into();
        let acl_permission_result: Result<(), &str> = acl_permission_result.into();
        acl_permission_result.map_err(|_e| RedisError::Str("User does not have permissions on key"))
    }
}

impl Drop for RedisUser {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_FreeModuleUser.unwrap()(self.user) };
    }
}

unsafe impl Send for RedisUser {}
