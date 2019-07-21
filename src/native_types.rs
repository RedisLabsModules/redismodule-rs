use std::cell::RefCell;
use std::ptr;
use crate::raw;

pub struct RedisType {
    pub name: &'static str,
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
}
