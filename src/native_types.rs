use std::cell::RefCell;
use std::ffi::CString;
use std::ptr;

use crate::raw;

pub struct RedisType {
    name: &'static str,
    version: i32,
    type_methods: raw::RedisModuleTypeMethods,
    pub raw_type: RefCell<*mut raw::RedisModuleType>,
}

// We want to be able to create static instances of this type,
// which means we need to implement Sync.
unsafe impl Sync for RedisType {}

impl RedisType {
    #[must_use]
    pub const fn new(
        name: &'static str,
        version: i32,
        type_methods: raw::RedisModuleTypeMethods,
    ) -> Self {
        Self {
            name,
            version,
            type_methods,
            raw_type: RefCell::new(ptr::null_mut()),
        }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn create_data_type(&self, ctx: *mut raw::RedisModuleCtx) -> Result<(), &str> {
        if self.name.len() != 9 {
            let msg = "Redis requires the length of native type names to be exactly 9 characters";
            redis_log(ctx, format!("{msg}, name is: '{}'", self.name).as_str());
            return Err(msg);
        }

        let type_name = CString::new(self.name).unwrap();

        let redis_type = unsafe {
            raw::RedisModule_CreateDataType.unwrap()(
                ctx,
                type_name.as_ptr(),
                self.version, // Encoding version
                &mut self.type_methods.clone(),
            )
        };

        if redis_type.is_null() {
            redis_log(ctx, "Error: created data type is null");
            return Err("Error: created data type is null");
        }

        *self.raw_type.borrow_mut() = redis_type;

        redis_log(
            ctx,
            format!("Created new data type '{}'", self.name).as_str(),
        );

        Ok(())
    }
}

// TODO: Move to raw
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn redis_log(ctx: *mut raw::RedisModuleCtx, msg: &str) {
    let level = CString::new("notice").unwrap(); // FIXME reuse this
    let msg = CString::new(msg).unwrap();
    unsafe {
        raw::RedisModule_Log.unwrap()(ctx, level.as_ptr(), msg.as_ptr());
    }
}
