use std::os::raw::c_int;
use std::str::Utf8Error;

use crate::raw;
use crate::{Context, RedisString};

/// A wrapper around the Redis Module Command Filter pointer.
///
/// This provides a type-safe way to work with command filter handles.
#[derive(Debug, Clone, Copy)]
pub struct CommandFilter {
    pub(crate) inner: *mut raw::RedisModuleCommandFilter,
}

// Required for thread-safe storage of command filters
unsafe impl Send for CommandFilter {}
unsafe impl Sync for CommandFilter {}

impl CommandFilter {
    /// Create a new CommandFilter from a raw pointer.
    pub fn new(inner: *mut raw::RedisModuleCommandFilter) -> Self {
        CommandFilter { inner }
    }

    /// Check if the filter pointer is null.
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    /// Get the raw pointer to the filter.
    ///
    /// This is useful when you need to store the filter handle and later
    /// recreate the CommandFilter wrapper.
    pub fn as_ptr(&self) -> *mut raw::RedisModuleCommandFilter {
        self.inner
    }
}

/// A wrapper around the Redis Module Command Filter Context.
///
/// This context is passed to command filter callbacks and provides methods
/// to inspect and modify command arguments.
pub struct CommandFilterContext {
    inner: *mut raw::RedisModuleCommandFilterCtx,
}

impl CommandFilterContext {
    /// Create a new CommandFilterContext from a raw pointer.
    ///
    /// # Safety
    /// The caller must ensure that the pointer is valid and only used within
    /// the lifetime of the command filter callback.
    pub fn new(inner: *mut raw::RedisModuleCommandFilterCtx) -> Self {
        CommandFilterContext { inner }
    }

    /// Get the number of arguments in the filtered command.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgsCount`.
    pub fn args_count(&self) -> c_int {
        unsafe { raw::RedisModule_CommandFilterArgsCount.unwrap()(self.inner) }
    }

    /// Get the argument at the specified position as a raw pointer.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgGet`.
    ///
    /// # Arguments
    /// * `pos` - The position of the argument (0-based)
    ///
    /// # Returns
    /// A pointer to the RedisModuleString, or null if the position is out of bounds.
    pub fn arg_get(&self, pos: c_int) -> *mut raw::RedisModuleString {
        unsafe { raw::RedisModule_CommandFilterArgGet.unwrap()(self.inner, pos) }
    }

    /// Get the argument at the specified position as a string slice.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgGet` with automatic conversion to `&str`.
    ///
    /// # Arguments
    /// * `pos` - The position of the argument (0-based)
    ///
    /// # Returns
    /// The argument as a string slice, or an error if the position is out of bounds
    /// or the argument is not valid UTF-8.
    pub fn arg_get_try_as_str(&self, pos: c_int) -> Result<&str, Utf8Error> {
        let arg = self.arg_get(pos);
        RedisString::from_ptr(arg)
    }

    /// Get the command name (the 0th argument) as a string slice.
    ///
    /// This is a convenience wrapper that always fetches argument 0, which is
    /// the command name.
    ///
    /// # Returns
    /// The command name as a string slice, or an error if not valid UTF-8.
    pub fn cmd_get_try_as_str(&self) -> Result<&str, Utf8Error> {
        self.arg_get_try_as_str(0)
    }

    /// Get all arguments except the command name.
    ///
    /// This is a convenience method that returns a vector of all arguments
    /// starting from position 1 (skipping the command name at position 0).
    ///
    /// # Returns
    /// A vector of string slices containing all arguments. Invalid UTF-8 arguments are skipped.
    pub fn get_all_args_wo_cmd(&self) -> Vec<&str> {
        let mut output = Vec::new();
        for pos in 1..self.args_count() {
            if let Ok(arg) = self.arg_get_try_as_str(pos) {
                output.push(arg);
            }
        }
        output
    }

    /// Replace the argument at the specified position.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgReplace`.
    ///
    /// # Arguments
    /// * `pos` - The position of the argument to replace (0-based)
    /// * `arg` - The new argument value as a string slice
    pub fn arg_replace(&self, pos: c_int, arg: &str) {
        unsafe {
            let new_arg = RedisString::create(None, arg);
            raw::string_retain_string(std::ptr::null_mut(), new_arg.inner);
            raw::RedisModule_CommandFilterArgReplace.unwrap()(self.inner, pos, new_arg.inner)
        };
    }

    /// Insert an argument at the specified position.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgInsert`.
    ///
    /// # Arguments
    /// * `pos` - The position where the argument should be inserted (0-based)
    /// * `arg` - The argument to insert as a string slice
    pub fn arg_insert(&self, pos: c_int, arg: &str) {
        unsafe {
            let new_arg = RedisString::create(None, arg);
            raw::string_retain_string(std::ptr::null_mut(), new_arg.inner);
            raw::RedisModule_CommandFilterArgInsert.unwrap()(self.inner, pos, new_arg.inner)
        };
    }

    /// Delete the argument at the specified position.
    ///
    /// Wrapper for `RedisModule_CommandFilterArgDelete`.
    ///
    /// # Arguments
    /// * `pos` - The position of the argument to delete (0-based)
    pub fn arg_delete(&self, pos: c_int) {
        unsafe { raw::RedisModule_CommandFilterArgDelete.unwrap()(self.inner, pos) };
    }

    /// Get the client ID of the client that issued the filtered command.
    ///
    /// Wrapper for `RedisModule_CommandFilterGetClientId`.
    ///
    /// # Returns
    /// The client ID as an unsigned 64-bit integer.
    ///
    /// # Note
    /// This API is not supported in Redis 7.0. It requires Redis 7.2 or later.
    #[cfg(any(
        feature = "min-redis-compatibility-version-7-4",
        feature = "min-redis-compatibility-version-7-2"
    ))]
    pub fn get_client_id(&self) -> u64 {
        unsafe { raw::RedisModule_CommandFilterGetClientId.unwrap()(self.inner) }
    }
}

impl Context {
    /// Register a command filter callback.
    ///
    /// Wrapper for `RedisModule_RegisterCommandFilter`.
    ///
    /// The callback will be invoked for each command executed. The callback
    /// should be an `extern "C"` function that accepts a command filter context.
    ///
    /// # Arguments
    /// * `callback` - The callback function to be invoked for each command
    /// * `flags` - Flags for the command filter (currently unused, pass 0)
    ///
    /// # Returns
    /// A CommandFilter handle that can be used to unregister the filter later.
    ///
    /// # Example
    /// ```no_run
    /// # use redis_module::{Context, RedisResult};
    /// # use redis_module::CommandFilterContext;
    /// extern "C" fn my_filter(fctx: *mut redis_module::raw::RedisModuleCommandFilterCtx) {
    ///     let filter_ctx = CommandFilterContext::new(fctx);
    ///     // Filter logic here
    /// }
    ///
    /// fn init(ctx: &Context) -> RedisResult {
    ///     let filter = ctx.register_command_filter(my_filter, 0);
    ///     // Store filter for later unregistration if needed
    ///     Ok(().into())
    /// }
    /// ```
    pub fn register_command_filter(
        &self,
        callback: extern "C" fn(*mut raw::RedisModuleCommandFilterCtx),
        flags: u32,
    ) -> CommandFilter {
        let filter_ptr = unsafe {
            raw::RedisModule_RegisterCommandFilter.unwrap()(
                self.ctx,
                Some(callback),
                flags as c_int,
            )
        };
        CommandFilter::new(filter_ptr)
    }

    /// Unregister a previously registered command filter.
    ///
    /// Wrapper for `RedisModule_UnregisterCommandFilter`.
    ///
    /// # Arguments
    /// * `filter` - The filter handle returned by `register_command_filter`
    pub fn unregister_command_filter(&self, filter: &CommandFilter) {
        unsafe {
            raw::RedisModule_UnregisterCommandFilter.unwrap()(self.ctx, filter.inner);
        }
    }
}
