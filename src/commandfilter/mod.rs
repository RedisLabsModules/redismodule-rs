use crate::raw;
use crate::RedisString;
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

    pub fn args_get(&self, pos: usize) -> RedisString {
        let arg = unsafe {
            raw::RedisModule_CommandFilterArgGet.unwrap()(self.ctx, pos as c_int)
                as *mut raw::RedisModuleString
        };
        RedisString::new(ptr::null_mut(), arg)
    }

    pub fn args_insert(&self, pos: usize, arg: RedisString) -> usize {
        // retain arg to since RedisModule_CommandFilterArgInsert going to release it too
        raw::string_retain_string(std::ptr::null_mut(), arg.inner);
        unsafe {
            raw::RedisModule_CommandFilterArgInsert.unwrap()(self.ctx, pos as c_int, arg.inner)
                as usize
        }
    }

    pub fn args_replace(&self, pos: usize, arg: RedisString) -> usize {
        // retain arg to since RedisModule_CommandFilterArgReplace going to release it too
        raw::string_retain_string(std::ptr::null_mut(), arg.inner);
        unsafe {
            raw::RedisModule_CommandFilterArgReplace.unwrap()(self.ctx, pos as c_int, arg.inner)
                as usize
        }
    }

    pub fn args_delete(&self, pos: usize) -> usize {
        unsafe { raw::RedisModule_CommandFilterArgDelete.unwrap()(self.ctx, pos as c_int) as usize }
    }
}
