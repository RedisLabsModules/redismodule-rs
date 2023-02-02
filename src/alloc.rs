use std::alloc::{GlobalAlloc, Layout};
use std::os::raw::c_void;

use crate::raw;

pub struct RedisAlloc;

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        /*
         * To make sure the memory allocation by Redis is aligned to the according to the layout,
         * we need to align the size of the allocation to the layout.
         *
         * "Memory is conceptually broken into equal-sized chunks,
         * where the chunk size is a power of two that is greater than the page size.
         * Chunks are always aligned to multiples of the chunk size.
         * This alignment makes it possible to find metadata for user objects very quickly."
         *
         * From: https://linux.die.net/man/3/jemalloc
         */
        let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));

        raw::RedisModule_Alloc.unwrap()(size).cast::<u8>()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        raw::RedisModule_Free.unwrap()(ptr.cast::<c_void>())
    }
}
