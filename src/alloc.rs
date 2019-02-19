use std::alloc::{GlobalAlloc, Layout, System};
use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, Ordering::SeqCst, ATOMIC_BOOL_INIT};

use crate::raw;

pub struct RedisAlloc;

static USE_REDIS_ALLOC: AtomicBool = ATOMIC_BOOL_INIT;

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let use_redis = USE_REDIS_ALLOC.load(SeqCst);
        if use_redis {
            return raw::RedisModule_Alloc.unwrap()(layout.size()) as *mut u8;
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
    eprintln!("Using Redis allocator");
    USE_REDIS_ALLOC.store(true, SeqCst);
}
