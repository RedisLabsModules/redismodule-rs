use std::ptr;
use std::cell::RefCell;
use std::ffi::CString;
use std::os::raw::{c_int, c_void};

use crate::raw;

pub struct RedisType {
    name: &'static str,
    pub raw_type: RefCell<*mut raw::RedisModuleType>,
}

// We want to be able to create static instances of this type,
// which means we need to implement Sync.
unsafe impl Sync for RedisType {}

impl RedisType {
    pub const fn new(name: &'static str) -> Self {
        RedisType {
            name,
            raw_type: RefCell::new(ptr::null_mut()),
        }
    }

    pub fn create_data_type(
        &self,
        ctx: *mut raw::RedisModuleCtx,
    ) -> Result<(), &str> {
        if self.name.len() != 9 {
            let msg = "Redis requires the length of native type names to be exactly 9 characters";
            redis_log(ctx, format!("{}, name is: '{}'", msg, self.name).as_str());
            return Err(msg);
        }

        let type_name = CString::new(self.name).unwrap();

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

        let redis_type = unsafe {
            raw::RedisModule_CreateDataType.unwrap()(
                ctx,
                type_name.as_ptr(),
                0, // Encoding version
                &mut type_methods,
            )
        };

        if redis_type.is_null() {
            redis_log(ctx, "Error: created data type is null");
            return Err("Error: created data type is null");
        }

        *self.raw_type.borrow_mut() = redis_type;

        redis_log(ctx, format!("Created new data type '{}'", self.name).as_str());

        Ok(())
    }
}

// FIXME: Generate these methods with a macro, since we need a set for each custom data type.

#[allow(non_snake_case,unused)]
#[no_mangle] // FIXME This should be unneeded
pub unsafe extern "C" fn MyTypeRdbLoad(
    rdb: *mut raw::RedisModuleIO,
    encver: c_int,
) -> *mut c_void {
//    eprintln!("MyTypeRdbLoad");
    ptr::null_mut()
}

#[allow(non_snake_case,unused)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeRdbSave(
    rdb: *mut raw::RedisModuleIO,
    value: *mut c_void,
) {
//    eprintln!("MyTypeRdbSave");
}

#[allow(non_snake_case,unused)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeAofRewrite(
    aof: *mut raw::RedisModuleIO,
    key: *mut raw::RedisModuleString,
    value: *mut c_void,
) {
//    eprintln!("MyTypeAofRewrite");
}

#[allow(non_snake_case,unused)]
#[no_mangle]
pub unsafe extern "C" fn MyTypeFree(
    value: *mut c_void,
) {
//    eprintln!("MyTypeFree");
}

// TODO: Move to raw
pub fn redis_log(
    ctx: *mut raw::RedisModuleCtx,
    msg: &str,
) {
    let level = CString::new("notice").unwrap(); // FIXME reuse this
    let msg = CString::new(msg).unwrap();
    unsafe {
        raw::RedisModule_Log.unwrap()(ctx, level.as_ptr(), msg.as_ptr());
    }
}

