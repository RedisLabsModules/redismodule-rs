use redis_module::{
    raw, redis_module, CommandFilter, CommandFilterContext, Context, NextArg, RedisError,
    RedisResult, RedisString, RedisValue,
};
use std::sync::atomic::{AtomicPtr, Ordering};

static COMMAND_FILTER: AtomicPtr<raw::RedisModuleCommandFilter> =
    AtomicPtr::new(std::ptr::null_mut());

extern "C" fn command_filter_callback(fctx: *mut raw::RedisModuleCommandFilterCtx) {
    let filter_ctx = CommandFilterContext::new(fctx);
    command_filter_impl(&filter_ctx);
}

fn command_filter_impl(fctx: &CommandFilterContext) {
    // Get the command name
    if let Ok(cmd_str) = fctx.cmd_get_try_as_str() {
        // Example: Log all SET commands
        if cmd_str.eq_ignore_ascii_case("set") {
            // You can inspect or modify arguments here
            // For example, you could replace sensitive data

            // Get all arguments (excluding command)
            let args = fctx.get_all_args_wo_cmd();
            let _num_args = args.len();

            // Note: In a real implementation, you would use the Context
            // to log, but we don't have access to it in the filter callback

            #[cfg(any(
                feature = "min-redis-compatibility-version-7-4",
                feature = "min-redis-compatibility-version-7-2"
            ))]
            {
                let _client_id = fctx.get_client_id();
            }
        }
    }
}

fn filter_register(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let current = COMMAND_FILTER.load(Ordering::Acquire);

    if !current.is_null() {
        return Err(RedisError::String("Filter already registered".to_string()));
    }

    let filter = ctx.register_command_filter(command_filter_callback, 0);
    COMMAND_FILTER.store(filter.as_ptr(), Ordering::Release);

    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn filter_unregister(ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    let filter_ptr = COMMAND_FILTER.swap(std::ptr::null_mut(), Ordering::AcqRel);

    if !filter_ptr.is_null() {
        let filter = CommandFilter::new(filter_ptr);
        ctx.unregister_command_filter(&filter);
        Ok(RedisValue::SimpleStringStatic("OK"))
    } else {
        Err(RedisError::String("No filter registered".to_string()))
    }
}

fn filter_test_args(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    let mut args_iter = args.into_iter().skip(1);
    let key = args_iter.next_arg()?;
    let value = args_iter.next_arg()?;

    // This SET command will be intercepted by the filter if it's registered
    ctx.call("SET", &[&key, &value])?;

    Ok(RedisValue::SimpleStringStatic("OK"))
}

fn filter_modify_example(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // This example demonstrates how to modify command arguments in a filter
    // In this case, we'll register a temporary filter that adds a prefix to SET keys

    extern "C" fn modify_filter(fctx: *mut raw::RedisModuleCommandFilterCtx) {
        let filter_ctx = CommandFilterContext::new(fctx);

        // Check if this is a SET command
        if let Ok(cmd) = filter_ctx.cmd_get_try_as_str() {
            if cmd.eq_ignore_ascii_case("set") && filter_ctx.args_count() >= 2 {
                // Get the current key
                if let Ok(key) = filter_ctx.arg_get_try_as_str(1) {
                    // Replace it with a prefixed version
                    let new_key = format!("filtered:{}", key);
                    filter_ctx.arg_replace(1, &new_key);
                }
            }
        }
    }

    let filter = ctx.register_command_filter(modify_filter, 0);

    // Execute a SET command which will be modified by the filter
    if args.len() > 2 {
        let _ = ctx.call("SET", &[&args[1], &args[2]]);
    }

    // Unregister the filter
    ctx.unregister_command_filter(&filter);

    Ok(RedisValue::SimpleStringStatic("OK"))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "command_filter",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["filter.register", filter_register, "", 0, 0, 0, ""],
        ["filter.unregister", filter_unregister, "", 0, 0, 0, ""],
        ["filter.test_args", filter_test_args, "", 0, 0, 0, ""],
        ["filter.modify_example", filter_modify_example, "", 0, 0, 0, ""],
    ],
}
