use std::alloc::{GlobalAlloc, Layout};
use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use crate::raw;

pub struct RedisAlloc;

static USE_REDIS_ALLOC: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));
        return raw::RedisModule_Alloc.unwrap()(size) as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        return raw::RedisModule_Free.unwrap()(ptr as *mut c_void);
    }
}

pub fn use_redis_alloc() {
    USE_REDIS_ALLOC.store(true, SeqCst);
    eprintln!("Now using Redis allocator");
}
