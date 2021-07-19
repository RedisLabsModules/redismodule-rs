use crate::raw;
use crate::RedisString;
use crate::Status;
use std::os::raw::c_int;
use std::ptr;

pub struct CommandFilterContext {
    pub ctx: *mut raw::RedisModuleCommandFilterCtx,
}

impl CommandFilterContext {
    pub fn new(ctx: *mut raw::RedisModuleCommandFilterCtx) -> Self {
        CommandFilterContext { ctx }
    }

    pub fn args_count(&self) -> usize {
        unsafe { raw::RedisModule_CommandFilterArgsCount.unwrap()(self.ctx) as usize }
    }

    pub fn args_get(&self, pos: usize) -> Option<RedisString> {
        let arg = unsafe {
            raw::RedisModule_CommandFilterArgGet.unwrap()(self.ctx, pos as c_int)
                as *mut raw::RedisModuleString
        };
        if arg.is_null() {
            None
        } else {
            Some(RedisString::new(ptr::null_mut(), arg))
        }
    }

    pub fn args_insert(&self, pos: usize, arg: RedisString) -> Status {
        // retain arg since RedisModule_CommandFilterArgInsert going to release it too
        raw::string_retain_string(std::ptr::null_mut(), arg.inner);
        let status = unsafe {
            raw::RedisModule_CommandFilterArgInsert.unwrap()(self.ctx, pos as c_int, arg.inner)
                .into()
        };

        // If the string wasn't inserted we have to release it ourself
        if status == Status::Err {
            unsafe { raw::RedisModule_FreeString.unwrap()(std::ptr::null_mut(), arg.inner) };
        }
        status
    }

    pub fn args_replace(&self, pos: usize, arg: RedisString) -> Status {
        // retain arg since RedisModule_CommandFilterArgReplace going to release it too
        raw::string_retain_string(std::ptr::null_mut(), arg.inner);
        let status = unsafe {
            raw::RedisModule_CommandFilterArgReplace.unwrap()(self.ctx, pos as c_int, arg.inner)
                .into()
        };

        // If the string wasn't replaced we have to release it ourself
        if status == Status::Err {
            unsafe { raw::RedisModule_FreeString.unwrap()(std::ptr::null_mut(), arg.inner) };
        }
        status
    }

    pub fn args_delete(&self, pos: usize) -> Status {
        unsafe { raw::RedisModule_CommandFilterArgDelete.unwrap()(self.ctx, pos as c_int).into() }
    }
}
