use std::alloc::{GlobalAlloc, Layout, System};
use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};

use crate::raw;

pub struct RedisAlloc;

static USE_REDIS_ALLOC: AtomicBool = AtomicBool::new(false);

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let use_redis = USE_REDIS_ALLOC.load(SeqCst);
        if use_redis {
            let size = (layout.size() + layout.align() - 1) & (!(layout.align() - 1));
            return raw::RedisModule_Alloc.unwrap()(size) as *mut u8;
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let use_redis = USE_REDIS_ALLOC.load(SeqCst);
        if use_redis {
            return raw::RedisModule_Free.unwrap()(ptr as *mut c_void);
        }
        System.dealloc(ptr, layout);
    }
}

pub fn use_redis_alloc() {
    USE_REDIS_ALLOC.store(true, SeqCst);
    eprintln!("Now using Redis allocator");
}
