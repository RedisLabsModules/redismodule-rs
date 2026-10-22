use std::{
    convert::Infallible,
    ffi::c_void,
    mem,
    ops::ControlFlow,
    ptr::{self},
};

use crate::{key::RedisKey, raw, RedisString};

/// The state handed to `RedisModule_ScanKey` as private data by the `try_*` methods: the user
/// callback plus the value it broke with, if it did.
struct TryScanState<B, F> {
    callback: F,
    broken: Option<B>,
}

/// A cursor to scan field/value pairs of a (hash) key.
///
/// It provides access via a closure given to [`ScanKeyCursor::for_each`] or if you need more control, you can use [`ScanKeyCursor::scan`]
/// and implement your own loop, e.g. to allow an early stop.
///
/// ## Example usage
///
/// Here we show how to extract values to communicate them back to the Redis client. We assume that the following hash key is setup before:
///
/// ```text
/// HSET user:123 name Alice age 29 location Austin
/// ```
///
/// The following example command implementation scans all fields and values in the hash key and returns them as an array of RedisString.
///
/// ```ignore
/// fn example_scan_key_for_each(ctx: &Context) -> RedisResult {
///    let key = ctx.open_key_with_flags("user:123", KeyFlags::NOEFFECTS | KeyFlags::NOEXPIRE | KeyFlags::ACCESS_EXPIRED );
///    let cursor  = ScanKeyCursor::new(key);
///
///    let res = RefCell::new(Vec::new());
///    cursor.for_each(|_key, field, value| {
///        let mut res = res.borrow_mut();
///        res.push(RedisValue::BulkRedisString(field.clone()));
///        res.push(RedisValue::BulkRedisString(value.clone()));
///    });
///
///    Ok(RedisValue::Array(res.take()))
/// }
/// ```
///
/// The method will produce the following output:
///
/// ```text
/// 1) "name"
/// 2) "Alice"
/// 3) "age"
/// 4) "29"
/// 5) "location"
/// 6) "Austin"
/// ```
pub struct ScanKeyCursor {
    key: RedisKey,
    inner_cursor: *mut raw::RedisModuleScanCursor,
}

impl ScanKeyCursor {
    /// Creates a new scan cursor for the given key.
    pub fn new(key: RedisKey) -> Self {
        let inner_cursor = unsafe { raw::RedisModule_ScanCursorCreate.unwrap()() };
        Self { key, inner_cursor }
    }

    /// Restarts the cursor from the beginning.
    pub fn restart(&self) {
        unsafe { raw::RedisModule_ScanCursorRestart.unwrap()(self.inner_cursor) };
    }

    /// Implements a call to `RedisModule_ScanKey` and calls the given closure for each callback invocation by ScanKey.
    /// Returns `true` if there are more fields to scan, `false` otherwise.
    ///
    /// The callback may be called multiple times per `RedisModule_ScanKey` invocation.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// while cursor.scan(|_key, field, value| {
    ///    // do something with field and value
    /// }) {
    ///   // do something between scans if needed, like an early stop
    /// }
    pub fn scan<F: FnMut(&RedisKey, &RedisString, &RedisString)>(&self, mut f: F) -> bool {
        let flow = self.try_scan(|key, field, value| {
            f(key, field, value);
            ControlFlow::<Infallible>::Continue(())
        });
        match flow {
            ControlFlow::Continue(more) => more,
            // `Infallible` is uninhabited, so the callback above can never have broken.
            ControlFlow::Break(never) => match never {},
        }
    }

    /// Implements a callback based for_each loop over all fields and values in the hash key.
    /// If you need more control, e.g. stopping after a scan invocation, then use [`ScanKeyCursor::scan`] directly.
    pub fn for_each<F: FnMut(&RedisKey, &RedisString, &RedisString)>(&self, mut f: F) {
        while self.scan(&mut f) {
            // do nothing, the callback does the work
        }
    }

    /// Like [`ScanKeyCursor::scan`], but the callback may return [`ControlFlow::Break`] to stop
    /// early. Returns `ControlFlow::Continue(true)` if there are more fields to scan,
    /// `ControlFlow::Continue(false)` if the key has been fully scanned, and `ControlFlow::Break`
    /// with the callback's value if the callback broke.
    ///
    /// # Breaking abandons the scan
    ///
    /// `RedisModule_ScanKey` cannot be aborted while it is running: it always finishes the batch
    /// of elements it is working on and advances the cursor past all of them. Breaking therefore
    /// only stops *your* callback from running; the elements after the break point in the current
    /// batch are skipped and will *not* be seen if you keep scanning with the same cursor.
    ///
    /// How much gets skipped depends on the encoding of the value. For hashtable and skiplist
    /// encodings a batch is a single hash bucket, but for the compact encodings (listpack,
    /// intset, ...) Redis walks the *entire* value in one invocation and marks the cursor as
    /// done. So on a small hash, breaking leaves the cursor exhausted, and a further
    /// [`ScanKeyCursor::try_for_each`] returns `Continue(())` immediately even though fields were
    /// never visited. Treat a break as abandoning the scan, and call [`ScanKeyCursor::restart`]
    /// if you need to go again.
    pub fn try_scan<B, F: FnMut(&RedisKey, &RedisString, &RedisString) -> ControlFlow<B>>(
        &self,
        callback: F,
    ) -> ControlFlow<B, bool> {
        unsafe extern "C" fn scan_callback<
            B,
            F: FnMut(&RedisKey, &RedisString, &RedisString) -> ControlFlow<B>,
        >(
            key: *mut raw::RedisModuleKey,
            field: *mut raw::RedisModuleString,
            value: *mut raw::RedisModuleString,
            data: *mut c_void,
        ) {
            // Safety: `data` is the `*mut TryScanState<B, F>` that `try_scan` derived from a
            // `&mut` to a live local, and Redis only calls us from inside that
            // `RedisModule_ScanKey` call.
            let state = unsafe { &mut *(data.cast::<TryScanState<B, F>>()) };

            // `RedisModuleScanKeyCB` returns `void`, so there is no way to tell Redis to stop
            // iterating. Once the callback has broken we simply do no more work, without even
            // constructing the wrapper types for the remaining elements of this invocation.
            if state.broken.is_some() {
                return;
            }

            let ctx = ptr::null_mut();
            let key = RedisKey::from_raw_parts(ctx, key);

            let field = RedisString::from_redis_module_string(ctx, field);
            let value = RedisString::from_redis_module_string(ctx, value);

            if let ControlFlow::Break(broken) = (state.callback)(&key, &field, &value) {
                state.broken = Some(broken);
            }

            // We don't own any of the passed in pointers, so we must ensure we don't run their destructors here
            mem::forget(field);
            mem::forget(value);
            mem::forget(key);
        }

        let mut state = TryScanState {
            callback,
            broken: None,
        };

        // Safety: The c-side initialized the function ptr and it is is never changed,
        // i.e. after module initialization the function pointers stay valid till the end of the program.
        let scan_key = unsafe { raw::RedisModule_ScanKey.unwrap() };

        let res = unsafe {
            scan_key(
                self.key.key_inner,
                self.inner_cursor,
                Some(scan_callback::<B, F>),
                (&mut state as *mut TryScanState<B, F>).cast::<c_void>(),
            )
        };

        match state.broken {
            Some(broken) => ControlFlow::Break(broken),
            None => ControlFlow::Continue(res != 0),
        }
    }

    /// Loops over all fields and values in the hash key until it is exhausted or the callback
    /// returns [`ControlFlow::Break`]. The analogue of [`Iterator::try_for_each`].
    ///
    /// See [`ScanKeyCursor::try_scan`] for what breaking does to the cursor, and the
    /// `scan_key_limit` command in `examples/scan_keys.rs` for a worked example.
    pub fn try_for_each<B, F: FnMut(&RedisKey, &RedisString, &RedisString) -> ControlFlow<B>>(
        &self,
        mut callback: F,
    ) -> ControlFlow<B> {
        while self.try_scan(&mut callback)? {}
        ControlFlow::Continue(())
    }
}

impl Drop for ScanKeyCursor {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_ScanCursorDestroy.unwrap()(self.inner_cursor) };
    }
}
