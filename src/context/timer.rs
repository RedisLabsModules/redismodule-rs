use std::convert::TryInto;
use std::ffi::c_void;
use std::time::Duration;

use crate::raw;
use crate::raw::RedisModuleTimerID;
use crate::{Context, RedisError};

// We use `repr(C)` since we access the underlying data field directly.
// The order matters: the data field must come first.
#[repr(C)]
struct CallbackData<F: FnOnce(&Context, T), T> {
    data: T,
    callback: F,
}

impl Context {
    /// Wrapper for `RedisModule_CreateTimer`.
    ///
    /// This function takes ownership of the provided data, and transfers it to Redis.
    /// The callback will get the original data back in a type safe manner.
    /// When the callback is done, the data will be dropped.
    pub fn create_timer<F, T>(&self, period: Duration, callback: F, data: T) -> RedisModuleTimerID
    where
        F: FnOnce(&Context, T),
    {
        let cb_data = CallbackData { data, callback };

        // Store the user-provided data on the heap before passing ownership of it to Redis,
        // so that it will outlive the current scope.
        let data = Box::from(cb_data);

        // Take ownership of the data inside the box and obtain a raw pointer to pass to Redis.
        let data = Box::into_raw(data);

        let timer_id = unsafe {
            raw::RedisModule_CreateTimer.unwrap()(
                self.ctx,
                period
                    .as_millis()
                    .try_into()
                    .expect("Value must fit in 64 bits"),
                Some(raw_callback::<F, T>),
                data as *mut c_void,
            )
        };

        timer_id
    }

    /// Wrapper for `RedisModule_StopTimer`.
    ///
    /// The caller is responsible for specifying the correct type for the returned data.
    /// This function has no way to know what the original type of the data was, so the
    /// same data type that was used for `create_timer` needs to be passed here to ensure
    /// their types are identical.
    pub fn stop_timer<T>(&self, timer_id: RedisModuleTimerID) -> Result<T, RedisError> {
        let mut data: *mut c_void = std::ptr::null_mut();

        let status: raw::Status =
            unsafe { raw::RedisModule_StopTimer.unwrap()(self.ctx, timer_id, &mut data) }.into();

        if status != raw::Status::Ok {
            return Err(RedisError::Str(
                "RedisModule_StopTimer failed, timer may not exist",
            ));
        }

        let data: T = take_data(data);
        return Ok(data);
    }

    /// Wrapper for `RedisModule_GetTimerInfo`.
    ///
    /// The caller is responsible for specifying the correct type for the returned data.
    /// This function has no way to know what the original type of the data was, so the
    /// same data type that was used for `create_timer` needs to be passed here to ensure
    /// their types are identical.
    pub fn get_timer_info<T>(
        &self,
        timer_id: RedisModuleTimerID,
    ) -> Result<(Duration, &T), RedisError> {
        let mut remaining: u64 = 0;
        let mut data: *mut c_void = std::ptr::null_mut();

        let status: raw::Status = unsafe {
            raw::RedisModule_GetTimerInfo.unwrap()(self.ctx, timer_id, &mut remaining, &mut data)
        }
        .into();

        if status != raw::Status::Ok {
            return Err(RedisError::Str(
                "RedisModule_GetTimerInfo failed, timer may not exist",
            ));
        }

        // Cast the *mut c_void supplied by the Redis API to a raw pointer of our custom type.
        let data = data as *mut T;

        // Dereference the raw pointer (we know this is safe, since Redis should return our
        // original pointer which we know to be good) and turn it into a safe reference
        let data = unsafe { &*data };

        Ok((Duration::from_millis(remaining), data))
    }
}

fn take_data<T>(data: *mut c_void) -> T {
    // Cast the *mut c_void supplied by the Redis API to a raw pointer of our custom type.
    let data = data as *mut T;

    // Take back ownership of the original boxed data, so we can unbox it safely.
    // If we don't do this, the data's memory will be leaked.
    let data = unsafe { Box::from_raw(data) };

    *data
}

extern "C" fn raw_callback<F, T>(ctx: *mut raw::RedisModuleCtx, data: *mut c_void)
where
    F: FnOnce(&Context, T),
{
    let ctx = &Context::new(ctx);

    if data.is_null() {
        ctx.log_debug("[callback] Data is null; this should not happen!");
        return;
    }

    let cb_data: CallbackData<F, T> = take_data(data);
    (cb_data.callback)(ctx, cb_data.data);
}
