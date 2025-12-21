use redis_module::{
    redis_module, BlockedClient, CallOptionResp, CallOptionsBuilder, CallReply, CallResult,
    Context, FutureCallReply, PromiseCallReply, RedisError, RedisResult, RedisString, RedisValue,
    ThreadSafeContext,
};

use std::thread;

fn call_test(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    let res: String = ctx.call("ECHO", &["TEST"])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str("Failed calling 'ECHO TEST'"));
    }

    let res: String = ctx.call("ECHO", vec!["TEST"].as_slice())?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic str vec",
        ));
    }

    let res: String = ctx.call("ECHO", &[b"TEST"])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with static [u8]",
        ));
    }

    let res: String = ctx.call("ECHO", vec![b"TEST"].as_slice())?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: String = ctx.call("ECHO", &[&"TEST".to_string()])?.try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str("Failed calling 'ECHO TEST' with String"));
    }

    let res: String = ctx
        .call("ECHO", vec![&"TEST".to_string()].as_slice())?
        .try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' dynamic &[u8] vec",
        ));
    }

    let res: String = ctx
        .call("ECHO", &[&ctx.create_string("TEST")])?
        .try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with RedisString",
        ));
    }

    let res: String = ctx
        .call("ECHO", vec![&ctx.create_string("TEST")].as_slice())?
        .try_into()?;
    if "TEST" != &res {
        return Err(RedisError::Str(
            "Failed calling 'ECHO TEST' with dynamic array of RedisString",
        ));
    }

    let call_options = CallOptionsBuilder::new().script_mode().errors_as_replies();
    let res: CallResult = ctx.call_ext::<&[&str; 0], _>("SHUTDOWN", &call_options.build(), &[]);
    if let Err(err) = res {
        let error_msg = err.to_utf8_string().unwrap();
        if !error_msg.contains("not allow") {
            return Err(RedisError::String(format!(
                "Failed to verify error messages, expected error message to contain 'not allow', error message: '{error_msg}'",
            )));
        }
    } else {
        return Err(RedisError::Str("Failed to set script mode on call_ext"));
    }

    // test resp3 on call_ext
    let call_options = CallOptionsBuilder::new()
        .script_mode()
        .resp(CallOptionResp::Resp3)
        .errors_as_replies()
        .build();
    ctx.call_ext::<_, CallResult>("HSET", &call_options, &["x", "foo", "bar"])
        .map_err(|e| -> RedisError { e.into() })?;
    let res: CallReply = ctx
        .call_ext::<_, CallResult>("HGETALL", &call_options, &["x"])
        .map_err(|e| -> RedisError { e.into() })?;
    if let CallReply::Map(map) = res {
        let res = map.iter().fold(Vec::new(), |mut vec, (key, val)| {
            if let CallReply::String(key) = key.unwrap() {
                vec.push(key.to_string().unwrap());
            }
            if let CallReply::String(val) = val.unwrap() {
                vec.push(val.to_string().unwrap());
            }
            vec
        });
        if res != vec!["foo".to_string(), "bar".to_string()] {
            return Err(RedisError::String(
                "Reply of hgetall does not match expected value".into(),
            ));
        }
    } else {
        return Err(RedisError::String(
            "Did not get a set type on hgetall".into(),
        ));
    }

    Ok("pass".into())
}

fn call_blocking_internal(ctx: &Context) -> PromiseCallReply {
    let call_options = CallOptionsBuilder::new().build_blocking();
    ctx.call_blocking("blpop", &call_options, &["list", "1"])
}

fn call_blocking_handle_future(ctx: &Context, f: FutureCallReply, blocked_client: BlockedClient) {
    let future_handler = f.set_unblock_handler(move |_ctx, reply| {
        let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);
        thread_ctx.reply(reply.map_or_else(|e| Err(e.into()), |v| Ok((&v).into())));
    });
    future_handler.dispose(ctx);
}

fn call_blocking(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    let res = call_blocking_internal(ctx);
    match res {
        PromiseCallReply::Resolved(r) => r.map_or_else(|e| Err(e.into()), |v| Ok((&v).into())),
        PromiseCallReply::Future(f) => {
            let blocked_client = ctx.block_client();
            call_blocking_handle_future(ctx, f, blocked_client);
            Ok(RedisValue::NoReply)
        }
    }
}

fn call_blocking_from_detach_ctx(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    let blocked_client = ctx.block_client();
    thread::spawn(move || {
        let ctx_guard = redis_module::MODULE_CONTEXT.lock();
        let res = call_blocking_internal(&ctx_guard);
        match res {
            PromiseCallReply::Resolved(r) => {
                let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);
                thread_ctx.reply(r.map_or_else(|e| Err(e.into()), |v| Ok((&v).into())));
            }
            PromiseCallReply::Future(f) => {
                call_blocking_handle_future(&ctx_guard, f, blocked_client);
            }
        }
    });
    Ok(RedisValue::NoReply)
}

fn call_dump_test(ctx: &Context, _: Vec<RedisString>) -> RedisResult {
    // Set a key with a value
    ctx.call("SET", &["test_dump_key", "test_value"])?;
    
    // Call DUMP which returns binary data (may not be valid UTF-8)
    let dump_result = ctx.call("DUMP", &["test_dump_key"])?;
    
    // Verify we got a result (should be StringBuffer for binary data)
    match dump_result {
        RedisValue::StringBuffer(data) => {
            if data.is_empty() {
                return Err(RedisError::Str("DUMP returned empty data"));
            }
        }
        RedisValue::SimpleString(_) => {
            // Also acceptable if the binary data happens to be valid UTF-8
        }
        _ => {
            return Err(RedisError::Str("DUMP returned unexpected type"));
        }
    }
    
    // Clean up
    ctx.call("DEL", &["test_dump_key"])?;
    
    Ok("pass".into())
}

//////////////////////////////////////////////////////

redis_module! {
    name: "call",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["call.test", call_test, "", 0, 0, 0, ""],
        ["call.blocking", call_blocking, "", 0, 0, 0, ""],
        ["call.blocking_from_detached_ctx", call_blocking_from_detach_ctx, "", 0, 0, 0, ""],
        ["call.dump_test", call_dump_test, "", 0, 0, 0, ""],
    ],
}
