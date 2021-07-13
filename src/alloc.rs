use std::alloc::{GlobalAlloc, Layout};
use std::os::raw::c_void;

use crate::raw;

pub struct RedisAlloc;

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));
        raw::RedisModule_Alloc.unwrap()(size).cast::<u8>()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        raw::RedisModule_Free.unwrap()(ptr.cast::<c_void>())
    }
}
