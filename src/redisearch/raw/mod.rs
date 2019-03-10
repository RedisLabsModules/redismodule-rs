#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

mod bindings;
pub use self::bindings::*;

use std::os::raw::c_int;

#[allow(improper_ctypes)]
#[link(name = "redisearch_api", kind = "static")]
extern "C" {
    pub fn Wrap_RediSearch_Initialize() -> c_int;
}
