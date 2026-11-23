use crate::context::Context;
use crate::key::RedisKey;
use crate::raw;
use crate::redismodule::RedisString;
use std::convert::Infallible;
use std::ffi::c_void;
use std::mem;
use std::ops::ControlFlow;
use std::ptr::NonNull;

pub struct KeysCursor {
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

/// The state handed to `RedisModule_Scan` as private data by the `try_*` methods: the user
/// callback plus the value it broke with, if it did.
struct TryScanState<B, F> {
    callback: F,
    broken: Option<B>,
}

extern "C" fn scan_callback<C: FnMut(&Context, &RedisString, Option<&RedisKey>)>(
    ctx: *mut raw::RedisModuleCtx,
    key_name: *mut raw::RedisModuleString,
    key: *mut raw::RedisModuleKey,
    private_data: *mut ::std::os::raw::c_void,
) {
    let context = Context::new(ctx);
    let key_name = RedisString::new(NonNull::new(ctx), key_name);
    let redis_key = if key.is_null() {
        None
    } else {
        // Safety: The returned `RedisKey` does not outlive this callbacks and so by necessity
        // the pointers passed in as parameters are valid for its entire lifetime.
        Some(unsafe { RedisKey::from_raw_parts(ctx, key) })
    };
    let callback = unsafe { &mut *(private_data.cast::<C>()) };
    callback(&context, &key_name, redis_key.as_ref());

    // We don't own any of the passed in pointers and have just created "temporary RAII types".
    // We must ensure we don't run their destructors here.
    mem::forget(redis_key);
    mem::forget(key_name);
}

/// `scan` cannot share this callback: it only holds a `&F`, so the `&F -> &mut F` cast it needs to
/// call the `FnMut` has to stay hidden behind the `*mut c_void` round trip through Redis.
extern "C" fn try_scan_callback<
    B,
    C: FnMut(&Context, &RedisString, Option<&RedisKey>) -> ControlFlow<B>,
>(
    ctx: *mut raw::RedisModuleCtx,
    key_name: *mut raw::RedisModuleString,
    key: *mut raw::RedisModuleKey,
    private_data: *mut ::std::os::raw::c_void,
) {
    // Safety: `private_data` is the `*mut TryScanState<B, C>` that `try_scan` derived from a
    // `&mut` to a live local, and Redis only calls us from inside that `RedisModule_Scan` call.
    let state = unsafe { &mut *(private_data.cast::<TryScanState<B, C>>()) };

    // `RedisModuleScanCB` returns `void`, so there is no way to tell Redis to stop iterating.
    // Once the callback has broken we simply do no more work, without even constructing the
    // wrapper types for the remaining keys of this invocation.
    if state.broken.is_some() {
        return;
    }

    let context = Context::new(ctx);
    let key_name = RedisString::new(NonNull::new(ctx), key_name);
    let redis_key = if key.is_null() {
        None
    } else {
        // Safety: The returned `RedisKey` does not outlive this callbacks and so by necessity
        // the pointers passed in as parameters are valid for its entire lifetime.
        Some(unsafe { RedisKey::from_raw_parts(ctx, key) })
    };

    if let ControlFlow::Break(value) = (state.callback)(&context, &key_name, redis_key.as_ref()) {
        state.broken = Some(value);
    }

    // We don't own any of the passed in pointers and have just created "temporary RAII types".
    // We must ensure we don't run their destructors here.
    mem::forget(redis_key);
    mem::forget(key_name);
}

impl KeysCursor {
    pub fn new() -> Self {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        Self { inner_cursor }
    }

    pub fn scan<F: FnMut(&Context, &RedisString, Option<&RedisKey>)>(
        &self,
        ctx: &Context,
        callback: &F,
    ) -> bool {
        let res = unsafe {
            raw::RedisModule_Scan.unwrap()(
                ctx.ctx,
                self.inner_cursor,
                Some(scan_callback::<F>),
                callback as *const F as *mut c_void,
            )
        };
        res != 0
    }

    /// Scans all keys of the current database, calling `callback` for each one.
    ///
    /// If you need to stop before the whole keyspace has been visited, use
    /// [`KeysCursor::try_for_each`] instead.
    pub fn for_each<F: FnMut(&Context, &RedisString, Option<&RedisKey>)>(
        &self,
        ctx: &Context,
        mut callback: F,
    ) {
        let flow = self.try_for_each(ctx, |ctx, key_name, key| {
            callback(ctx, key_name, key);
            ControlFlow::<Infallible>::Continue(())
        });
        match flow {
            ControlFlow::Continue(()) => (),
            // `Infallible` is uninhabited, so the callback above can never have broken.
            ControlFlow::Break(never) => match never {},
        }
    }

    /// Like [`KeysCursor::scan`], but the callback may return [`ControlFlow::Break`] to stop
    /// early. Returns `ControlFlow::Continue(true)` if there are more keys to scan,
    /// `ControlFlow::Continue(false)` if the keyspace has been fully scanned, and
    /// `ControlFlow::Break` with the callback's value if the callback broke.
    ///
    /// # Breaking abandons the scan
    ///
    /// `RedisModule_Scan` cannot be aborted while it is running: it always finishes the batch of
    /// keys it is working on and advances the cursor past all of them. Breaking therefore only
    /// stops *your* callback from running; the keys after the break point in the current batch
    /// are skipped and will *not* be seen if you keep scanning with the same cursor. Treat a
    /// break as abandoning the scan, and call [`KeysCursor::restart`] if you need to go again.
    pub fn try_scan<B, F: FnMut(&Context, &RedisString, Option<&RedisKey>) -> ControlFlow<B>>(
        &self,
        ctx: &Context,
        callback: F,
    ) -> ControlFlow<B, bool> {
        let mut state = TryScanState {
            callback,
            broken: None,
        };
        let res = unsafe {
            raw::RedisModule_Scan.unwrap()(
                ctx.ctx,
                self.inner_cursor,
                Some(try_scan_callback::<B, F>),
                (&mut state as *mut TryScanState<B, F>).cast::<c_void>(),
            )
        };
        match state.broken {
            Some(value) => ControlFlow::Break(value),
            None => ControlFlow::Continue(res != 0),
        }
    }

    /// Scans all keys of the current database, calling `callback` for each one, until the
    /// keyspace is exhausted or the callback returns [`ControlFlow::Break`]. The analogue of
    /// [`Iterator::try_for_each`].
    ///
    /// See [`KeysCursor::try_scan`] for what breaking does to the cursor, and the `scan_keys_limit`
    /// command in `examples/scan_keys.rs` for a worked example.
    pub fn try_for_each<
        B,
        F: FnMut(&Context, &RedisString, Option<&RedisKey>) -> ControlFlow<B>,
    >(
        &self,
        ctx: &Context,
        mut callback: F,
    ) -> ControlFlow<B> {
        while self.try_scan(ctx, &mut callback)? {}
        ControlFlow::Continue(())
    }

    pub fn restart(&self) {
        unsafe { raw::RedisModule_ScanCursorRestart.unwrap()(self.inner_cursor) };
    }
}

impl Default for KeysCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for KeysCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
