use std::alloc::{GlobalAlloc, Layout};
use std::ptr;

use crate::raw;

/// Panics with a message without using an allocator.
/// Useful when using the allocator should be avoided or it is
/// inaccessible. The default [std::panic] performs allocations and so
/// will cause a double panic without a meaningful message if the
/// allocator can't be used. This function makes sure we can panic with
/// a reasonable message even without the allocator working.
fn allocation_free_panic(message: &'static str) -> ! {
    use std::os::unix::io::AsRawFd;

    let _ = nix::unistd::write(std::io::stderr().as_raw_fd(), message.as_bytes());

    std::process::abort();
}

const REDIS_ALLOCATOR_NOT_AVAILABLE_MESSAGE: &str =
    "Critical error: the Redis Allocator isn't available.\n";

/// The alignment `RedisModule_Alloc` provides on its own.
///
/// It is Redis' `zmalloc`, which forwards to an allocator handing out
/// `max_align_t`-aligned addresses — jemalloc, tcmalloc or libc `malloc`
/// depending on how the server was built.
///
/// Understating this costs an unnecessary fixup; overstating it hands out
/// under-aligned memory, so it is the weakest guarantee those allocators share
/// rather than the strongest any of them happens to provide.
const MIN_ALIGN: usize = 2 * std::mem::size_of::<usize>();

/// Size of the header kept directly below an over-aligned block, holding the
/// pointer that has to be given back to `RedisModule_Free`.
const HEADER: usize = std::mem::size_of::<*mut u8>();

/// Defines the Redis allocator. This allocator delegates the allocation
/// and deallocation tasks to the Redis server when available, otherwise
/// it panics.
#[derive(Default, Debug, Copy, Clone)]
pub struct RedisAlloc;

unsafe impl GlobalAlloc for RedisAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let Some(alloc) = raw::RedisModule_Alloc else {
            allocation_free_panic(REDIS_ALLOCATOR_NOT_AVAILABLE_MESSAGE)
        };

        alloc_aligned(layout, |size| alloc(size).cast())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let Some(free) = raw::RedisModule_Free else {
            allocation_free_panic(REDIS_ALLOCATOR_NOT_AVAILABLE_MESSAGE)
        };

        dealloc_aligned(ptr, layout, |ptr| free(ptr.cast()));
    }
}

/// Allocate `layout` through `alloc`, which is expected to behave like
/// `RedisModule_Alloc`: it takes a size, and returns a block aligned to at most
/// [`MIN_ALIGN`].
///
/// Layouts within [`MIN_ALIGN`] are served by `alloc` directly. A stricter one
/// cannot be — the module API has no aligned-allocation primitive — so it is
/// carved out of a larger block instead, with the underlying pointer stored in
/// the [`HEADER`] word below the address returned to the caller.
///
/// Returns a null pointer if `alloc` does, or if the padded size overflows.
///
/// Taking the allocator as a closure keeps this testable without a running
/// Redis server behind `RedisModule_Alloc`.
///
/// # Safety
///
/// `alloc` must behave like [`GlobalAlloc::alloc`] for the size it is given.
unsafe fn alloc_aligned(layout: Layout, alloc: impl FnOnce(usize) -> *mut u8) -> *mut u8 {
    if layout.align() <= MIN_ALIGN {
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

        return alloc(size);
    }

    // Worst case the block starts one byte past an aligned address, so reaching
    // the next one from just above the header costs `align - 1` further bytes.
    let Some(size) = layout.size().checked_add(layout.align() + HEADER) else {
        return ptr::null_mut();
    };

    let base = alloc(size);
    if base.is_null() {
        return base;
    }

    let offset = (base.addr() + HEADER).next_multiple_of(layout.align()) - base.addr();
    let ptr = base.add(offset);
    // Storing the pointer rather than the offset keeps the provenance Redis gave
    // us, so `dealloc_aligned` hands back exactly what `alloc` returned.
    ptr.cast::<*mut u8>().sub(1).write(base);

    ptr
}

/// Free a pointer produced by [`alloc_aligned`] through the matching `free`.
///
/// # Safety
///
/// `ptr` must have come from [`alloc_aligned`] with this same `layout`, and
/// `free` must release blocks allocated by the closure that call was given.
unsafe fn dealloc_aligned(ptr: *mut u8, layout: Layout, free: impl FnOnce(*mut u8)) {
    let ptr = if layout.align() <= MIN_ALIGN {
        ptr
    } else {
        // `alloc_aligned` took the same branch for this layout, so the word below
        // `ptr` is inside the allocation and holds the underlying pointer.
        ptr.cast::<*mut u8>().sub(1).read()
    };

    free(ptr);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::alloc::System;
    use std::cell::Cell;

    /// A block handed out by [`Zmalloc`], and what it takes to give it back.
    #[derive(Clone, Copy)]
    struct Block {
        /// What `System` returned, along with the layout it has to be given back
        /// with.
        base: *mut u8,
        layout: Layout,
        /// What [`Zmalloc::alloc`] returned, and so what [`Zmalloc::free`] has to
        /// be handed.
        handed_out: *mut u8,
        /// The size [`alloc_aligned`] asked for, before this double padded it.
        requested: usize,
    }

    /// Stands in for `RedisModule_Alloc`, which cannot be called without a
    /// running server. Like `zmalloc` it takes only a size, and it returns the
    /// weakest thing `zmalloc` may return: a block aligned to exactly
    /// [`MIN_ALIGN`] and never more.
    ///
    /// That last part is deliberate. Real allocators over-align by chance often
    /// enough that under-aligned allocations go unnoticed on Linux for years, and
    /// a double inheriting that luck would pass these tests against an
    /// implementation that never aligns anything.
    #[derive(Default)]
    struct Zmalloc {
        last: Cell<Option<Block>>,
    }

    impl Zmalloc {
        fn alloc(&self, requested: usize) -> *mut u8 {
            // Room to push the block off a stricter boundary when it lands on one.
            let layout = Layout::from_size_align(requested + MIN_ALIGN, MIN_ALIGN).unwrap();
            let base = unsafe { System.alloc(layout) };

            let offset = if base.addr() % (2 * MIN_ALIGN) == 0 {
                MIN_ALIGN
            } else {
                0
            };
            let handed_out = unsafe { base.add(offset) };
            assert_eq!(
                handed_out.addr() % (2 * MIN_ALIGN),
                MIN_ALIGN,
                "the double is over-aligning, so it no longer models the worst case"
            );

            self.last.set(Some(Block {
                base,
                layout,
                handed_out,
                requested,
            }));

            handed_out
        }

        fn free(&self, ptr: *mut u8) {
            let block = self.last();
            assert_eq!(
                ptr, block.handed_out,
                "freed a pointer that was never allocated"
            );
            unsafe { System.dealloc(block.base, block.layout) };
        }

        fn last(&self) -> Block {
            self.last.get().unwrap()
        }
    }

    /// Alignments above [`MIN_ALIGN`] worth covering: the 32 bytes an AVX2
    /// searcher needs, plus a cache line and a page.
    const OVER_ALIGNED: [usize; 3] = [32, 64, 4096];

    const _: () = assert!(OVER_ALIGNED[0] > MIN_ALIGN, "these no longer over-align");

    #[test]
    fn layouts_within_min_align_are_served_directly() {
        // Every alignment the fast path claims, derived rather than listed: on a
        // target where `MIN_ALIGN` is 8 a hard-coded 16 would exercise the other
        // branch instead.
        for align in (0..=MIN_ALIGN.ilog2()).map(|shift| 1usize << shift) {
            let zmalloc = Zmalloc::default();
            let layout = Layout::from_size_align(24, align).unwrap();

            let ptr = unsafe { alloc_aligned(layout, |size| zmalloc.alloc(size)) };

            assert!(!ptr.is_null());
            assert_eq!(ptr, zmalloc.last().handed_out, "align {align} was adjusted");

            unsafe { dealloc_aligned(ptr, layout, |ptr| zmalloc.free(ptr)) };
        }
    }

    #[test]
    fn a_request_smaller_than_its_alignment_is_rounded_up() {
        let zmalloc = Zmalloc::default();
        let layout = Layout::from_size_align(8, MIN_ALIGN).unwrap();

        let ptr = unsafe { alloc_aligned(layout, |size| zmalloc.alloc(size)) };

        // The rounding is what lifts the request into a size class the allocator
        // returns suitably aligned; without it a `malloc(8)` may only be
        // 8-aligned.
        assert_eq!(zmalloc.last().requested, MIN_ALIGN);

        unsafe { dealloc_aligned(ptr, layout, |ptr| zmalloc.free(ptr)) };
    }

    #[test]
    fn over_aligned_blocks_are_aligned_and_usable() {
        for align in OVER_ALIGNED {
            for size in [1, align, 7 * align] {
                let zmalloc = Zmalloc::default();
                let layout = Layout::from_size_align(size, align).unwrap();

                let ptr = unsafe { alloc_aligned(layout, |size| zmalloc.alloc(size)) };

                assert!(!ptr.is_null());
                assert_eq!(ptr.addr() % align, 0, "size {size} align {align}");

                // Touching every byte proves the block really extends that far.
                // Miri and the sanitizers fail the test if it does not.
                unsafe { ptr.write_bytes(0xAB, size) };

                unsafe { dealloc_aligned(ptr, layout, |ptr| zmalloc.free(ptr)) };
            }
        }
    }

    #[test]
    fn over_aligned_blocks_are_freed_at_the_underlying_pointer() {
        let zmalloc = Zmalloc::default();
        let layout = Layout::from_size_align(100, 64).unwrap();

        let ptr = unsafe { alloc_aligned(layout, |size| zmalloc.alloc(size)) };
        assert_ne!(
            ptr,
            zmalloc.last().handed_out,
            "test needs a layout that gets adjusted"
        );

        // `Zmalloc::free` asserts it was handed its own pointer back.
        unsafe { dealloc_aligned(ptr, layout, |ptr| zmalloc.free(ptr)) };
    }

    #[test]
    fn a_failed_allocation_propagates_null() {
        let layout = Layout::from_size_align(64, 64).unwrap();

        let ptr = unsafe { alloc_aligned(layout, |_| ptr::null_mut()) };

        // Null must come straight back out: writing a header below it would
        // dereference the null pointer.
        assert!(ptr.is_null());
    }
}
