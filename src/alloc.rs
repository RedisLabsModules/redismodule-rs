use std::alloc::{GlobalAlloc, Layout};
use std::os::raw::c_void;

use crate::raw;

pub struct RedisAlloc;

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));
        return raw::RedisModule_Alloc.unwrap()(size) as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        return raw::RedisModule_Free.unwrap()(ptr as *mut c_void);
    }
}
