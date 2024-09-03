use std::alloc::Layout;

use crate::{
    raw, Context, RedisModule_DefragAlloc, RedisModule_DefragCursorGet,
    RedisModule_DefragCursorSet, RedisModule_DefragRedisModuleString, RedisModule_DefragShouldStop,
    RedisString, Status,
};
use crate::{RedisError, RedisLockIndicator};
use linkme::distributed_slice;

pub struct DefragContext {
    defrag_ctx: *mut raw::RedisModuleDefragCtx,
}

/// Having a [DefragContext] is indication that we are
/// currently holding the Redis GIL, this is why it is safe to
/// implement a [RedisLockIndicator] for [DefragContext].
unsafe impl RedisLockIndicator for DefragContext {}

impl DefragContext {
    /// Creates a new [`DefragContext`] from a poiter to [`raw::RedisModuleDefragCtx`].
    /// The function is considered unsafe because the provided pointer
    /// must be a valid pointer to [`raw::RedisModuleDefragCtx`], and the Redis GIL must be held.
    /// The function is exposed for users that wants to implement the defrag function
    /// on their module datatype, they can use this function to create [`DefragContext`]
    /// that can be used in a safely manner.
    /// Notice that the returned [`DefragContext`] borrows the pointer to [`raw::RedisModuleDefragCtx`]
    /// so it can not outlive it (this means that it should not be used once the defrag callback ends).
    pub unsafe fn new(defrag_ctx: *mut raw::RedisModuleDefragCtx) -> DefragContext {
        DefragContext { defrag_ctx }
    }

    /// When the data type defrag callback iterates complex structures, this
    /// function should be called periodically. A [`false`] return
    /// indicates the callback may continue its work. A [`true`]
    /// indicates it should stop.
    ///
    /// When stopped, the callback may use [`Self::set_cursor`] to store its
    /// position so it can later use [`Self::get_cursor`] to resume defragging.
    ///
    /// When stopped and more work is left to be done, the callback should
    /// return 1. Otherwise, it should return 0.
    ///
    /// NOTE: Modules should consider the frequency in which this function is called,
    /// so it generally makes sense to do small batches of work in between calls.
    pub fn should_stop(&self) -> bool {
        let should_stop = unsafe {
            RedisModule_DefragShouldStop.expect("RedisModule_DefragShouldStop is NULL")(
                self.defrag_ctx,
            )
        };
        should_stop != 0
    }

    /// Store an arbitrary cursor value for future re-use.
    ///
    /// This should only be called if [`Self::should_stop`] has returned a non-zero
    /// value and the defrag callback is about to exit without fully iterating its
    /// data type.
    ///
    /// This behavior is reserved to cases where late defrag is performed. Late
    /// defrag is selected for keys that implement the `free_effort` callback and
    /// return a `free_effort` value that is larger than the defrag
    /// 'active-defrag-max-scan-fields' configuration directive.
    ///
    /// Smaller keys, keys that do not implement `free_effort` or the global
    /// defrag callback are not called in late-defrag mode. In those cases, a
    /// call to this function will return [`Status::Err`].
    ///
    /// The cursor may be used by the module to represent some progress into the
    /// module's data type. Modules may also store additional cursor-related
    /// information locally and use the cursor as a flag that indicates when
    /// traversal of a new key begins. This is possible because the API makes
    /// a guarantee that concurrent defragmentation of multiple keys will
    /// not be performed.
    pub fn set_cursor(&self, cursor: u64) -> Status {
        unsafe {
            RedisModule_DefragCursorSet.expect("RedisModule_DefragCursorSet is NULL")(
                self.defrag_ctx,
                cursor,
            )
        }
        .into()
    }

    /// Fetch a cursor value that has been previously stored using [`Self::set_cursor`].
    /// If not called for a late defrag operation, [`Err`] will be returned.
    pub fn get_cursor(&self) -> Result<u64, RedisError> {
        let mut cursor: u64 = 0;
        let res: Status = unsafe {
            RedisModule_DefragCursorGet.expect("RedisModule_DefragCursorGet is NULL")(
                self.defrag_ctx,
                (&mut cursor) as *mut u64,
            )
        }
        .into();
        if res == Status::Ok {
            Ok(cursor)
        } else {
            Err(RedisError::Str("Could not get cursor value"))
        }
    }

    /// Defrag a memory allocation previously allocated by RM_Alloc, RM_Calloc, etc.
    /// The defragmentation process involves allocating a new memory block and copying
    /// the contents to it, like realloc().
    ///
    /// If defragmentation was not necessary, NULL is returned and the operation has
    /// no other effect.
    ///
    /// If a non-NULL value is returned, the caller should use the new pointer instead
    /// of the old one and update any reference to the old pointer, which must not
    /// be used again.
    ///
    /// The function is unsafe because it is assumed that the pointer is valid and previusly
    /// allocated. It is considered undefined if this is not the case.
    pub unsafe fn defrag_realloc<T>(&self, mut ptr: *mut T) -> *mut T {
        let new_ptr: *mut T = RedisModule_DefragAlloc.expect("RedisModule_DefragAlloc is NULL")(
            self.defrag_ctx,
            ptr.cast(),
        )
        .cast();
        if !new_ptr.is_null() {
            ptr = new_ptr;
        }
        ptr
    }

    /// Allocate memory using defrag allocator if supported by the
    /// current Redis server, fallback to regular allocation otherwise.
    pub fn defrag_alloc<T>(&self, layout: Layout) -> *mut T {
        unsafe { std::alloc::alloc(layout) }.cast()
    }

    /// Deallocate memory using defrag deallocator if supported by the
    /// current Redis server, fallback to regular deallocation otherwise.
    pub fn defrag_dealloc<T>(&self, ptr: *mut T, layout: Layout) {
        unsafe { std::alloc::dealloc(ptr.cast(), layout) }
    }

    /// Defrag a [RedisString]
    ///
    /// NOTE: It is only possible to defrag strings that have a single reference.
    /// Typically this means strings that was copy/cloned using [RedisString::safe_clone]
    /// or created using [RedisString::new] will not be defrag and will be returned as is.
    pub fn defrag_redis_string(&self, mut s: RedisString) -> RedisString {
        let new_inner = unsafe {
            RedisModule_DefragRedisModuleString
                .expect("RedisModule_DefragRedisModuleString is NULL")(
                self.defrag_ctx, s.inner
            )
        };
        if !new_inner.is_null() {
            s.inner = new_inner;
        }
        s
    }
}

#[distributed_slice()]
pub static DEFRAG_FUNCTIONS_LIST: [fn(&DefragContext)] = [..];

#[distributed_slice()]
pub static DEFRAG_START_FUNCTIONS_LIST: [fn(&DefragContext)] = [..];

#[distributed_slice()]
pub static DEFRAG_END_FUNCTIONS_LIST: [fn(&DefragContext)] = [..];

extern "C" fn defrag_function(defrag_ctx: *mut raw::RedisModuleDefragCtx) {
    let mut ctx = DefragContext { defrag_ctx };
    DEFRAG_FUNCTIONS_LIST.iter().for_each(|callback| {
        callback(&mut ctx);
    });
}

extern "C" fn defrag_start_function(defrag_ctx: *mut raw::RedisModuleDefragCtx) {
    let mut ctx = DefragContext { defrag_ctx };
    DEFRAG_START_FUNCTIONS_LIST.iter().for_each(|callback| {
        callback(&mut ctx);
    });
}

extern "C" fn defrag_end_function(defrag_ctx: *mut raw::RedisModuleDefragCtx) {
    let mut ctx = DefragContext { defrag_ctx };
    DEFRAG_END_FUNCTIONS_LIST.iter().for_each(|callback| {
        callback(&mut ctx);
    });
}

pub fn register_defrag_functions(ctx: &Context) -> Result<(), RedisError> {
    let register_defrag_function = match unsafe { raw::RedisModule_RegisterDefragFunc } {
        Some(f) => f,
        None => {
            ctx.log_warning("Skip register defrag function as defrag is not supported on the current Redis server.");
            return Ok(());
        }
    };
    if !DEFRAG_FUNCTIONS_LIST.is_empty() {
        let res = unsafe { register_defrag_function(ctx.ctx, Some(defrag_function)) };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str("Failed register defrag function"));
        }
    }

    let register_defrag_callbacks = match unsafe { raw::RedisModule_RegisterDefragCallbacks } {
        Some(f) => f,
        None => {
            ctx.log_warning("Skip register defrag callbacks as defrag callbacks is not supported on the current Redis server.");
            return Ok(());
        }
    };
    if !DEFRAG_START_FUNCTIONS_LIST.is_empty() || !DEFRAG_END_FUNCTIONS_LIST.is_empty() {
        let res = unsafe {
            register_defrag_callbacks(
                ctx.ctx,
                Some(defrag_start_function),
                Some(defrag_end_function),
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str("Failed register defrag callbacks"));
        }
    }

    Ok(())
}
