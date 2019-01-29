use core::ptr;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};

use crate::raw;

pub struct RedisModuleType<'a> {
    name: &'a str,
    raw_type: *mut raw::RedisModuleType,
}

// We want to be able to create static instances of this type,
// which means we need to implement Sync.
unsafe impl<'a> Sync for RedisModuleType<'a> {}

fn redis_log(
    ctx: *mut raw::RedisModuleCtx,
    msg: &str,
) {
    let level = CString::new("notice").unwrap(); // FIXME reuse this
    let msg = CString::new(msg).unwrap();
    unsafe {
        raw::RedisModule_Log.unwrap()(ctx, level.as_ptr(), msg.as_ptr());
    }
}

impl<'a> RedisModuleType<'a> {
    pub const fn new(name: &'a str) -> Self {
        RedisModuleType {
            name,
            raw_type: ptr::null_mut(),
        }
    }

    pub fn create_data_type(
        &mut self,
        ctx: *mut raw::RedisModuleCtx,
    ) -> Result<(), ()> {
        let type_name = CString::new(self.name).unwrap();

        redis_log(ctx, "Here 1");

        let mut type_methods = raw::RedisModuleTypeMethods {
            version: raw::REDISMODULE_TYPE_METHOD_VERSION as u64,

            rdb_load: Some(MyTypeRdbLoad),
            rdb_save: Some(MyTypeRdbSave),
            aof_rewrite: Some(MyTypeAofRewrite),
            free: Some(MyTypeFree),

            // Currently unused by Redis
            mem_usage: None,
            digest: None,
        };

        redis_log(ctx, "Here 2");

        let redis_type = unsafe {
            raw::RedisModule_CreateDataType.unwrap()(
                ctx,
                type_name.as_ptr(),
                0, // Encoding version
                &mut type_methods,
            )
        };

        redis_log(ctx, "Here 3");

        if redis_type.is_null() {
            redis_log(ctx, "Error: created data type is null");
            return Err(());
        }

        redis_log(ctx, "Here 4");

        self.raw_type = redis_type;

        redis_log(ctx, "Here 5");

        Ok(())
    }
}

// FIXME: Generate these methods with a macro, since we need a set for each custom data type.

#[allow(non_snake_case)]
#[no_mangle] // FIXME This should be unneeded
pub unsafe extern "C" fn MyTypeRdbLoad(
    rdb: *mut raw::RedisModuleIO,
    encver: c_int,
) -> *mut c_void {
//    eprintln!("MyTypeRdbLoad");
    ptr::null_mut()
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeRdbSave(
    rdb: *mut raw::RedisModuleIO,
    value: *mut c_void,
) {
//    eprintln!("MyTypeRdbSave");
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeAofRewrite(
    aof: *mut raw::RedisModuleIO,
    key: *mut raw::RedisModuleString,
    value: *mut c_void,
) {
//    eprintln!("MyTypeAofRewrite");
}

#[allow(non_snake_case)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeFree(
    value: *mut c_void,
) {
//    eprintln!("MyTypeFree");
}

