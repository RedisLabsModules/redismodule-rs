use std::alloc::{GlobalAlloc, Layout};

use crate::raw;

/// Defines the Redis allocator. This allocator delegates the allocation
/// and deallocation tasks to the Redis server when available, otherwise
/// it fallbacks to the default Rust [std::alloc::System] allocator
/// which is always available compared to the Redis allocator.
#[derive(Copy, Clone)]
pub struct RedisAlloc {
    system: std::alloc::System,
}

impl RedisAlloc {
    pub const fn new() -> Self {
        Self {
            system: std::alloc::System,
        }
    }
}

impl Default for RedisAlloc {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        match raw::RedisModule_Alloc {
            Some(alloc) => {
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
                alloc(size).cast()
            }
            None => self.system.alloc(layout).cast(),
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        match raw::RedisModule_Free {
            Some(dealloc) => dealloc(ptr.cast()),
            None => self.system.dealloc(ptr, layout),
        }
    }
}
