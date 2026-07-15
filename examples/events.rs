use redis_module::{
    raw, redis_module, Context, NotifyEvent, RedisError, RedisResult, RedisString, RedisValue,
    Status,
};
use std::ffi::CString;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

static NUM_KEY_MISSES: AtomicI64 = AtomicI64::new(0);
static NUM_KEYS: AtomicI64 = AtomicI64::new(0);
static LAST_GENERIC_EVENT: Mutex<String> = Mutex::new(String::new());

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    if key == b"num_sets" {
        // break infinit look
        return;
    }
    let msg = format!(
        "Received event: {:?} on key: {} via event: {}",
        event_type,
        std::str::from_utf8(key).unwrap(),
        event
    );
    ctx.log_notice(msg.as_str());
    let _ = ctx.add_post_notification_job(|ctx| {
        // it is not safe to write inside the notification callback itself.
        // So we perform the write on a post job notificaiton.
        if let Err(e) = ctx.call("incr", &["num_sets"]) {
            ctx.log_warning(&format!("Error on incr command, {}.", e));
        }
    });
}

fn on_stream(ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &[u8]) {
    ctx.log_debug("Stream event received!");
}

fn event_send(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() > 1 {
        return Err(RedisError::WrongArity);
    }

    let key_name = RedisString::create(NonNull::new(ctx.ctx), "mykey");
    let status = ctx.notify_keyspace_event(NotifyEvent::GENERIC, "events.send", &key_name);
    match status {
        Status::Ok => Ok("Event sent".into()),
        Status::Err => Err(RedisError::Str("Generic error")),
    }
}

fn on_generic(_ctx: &Context, _event_type: NotifyEvent, event: &str, _key: &[u8]) {
    *LAST_GENERIC_EVENT.lock().unwrap() = event.to_string();
}

fn event_send_invalid_utf8(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() > 1 {
        return Err(RedisError::WrongArity);
    }

    let key_name = RedisString::create(NonNull::new(ctx.ctx), "mykey");
    // Fire a keyspace event whose name is not valid UTF-8 (0xFF can never
    // appear in a UTF-8 string), the way a C module could. This has to go
    // through the raw API because the safe wrapper only accepts &str.
    let event = CString::new(&b"ev\xFFnt"[..]).unwrap();
    // SAFETY: `RedisModule_NotifyKeyspaceEvent` is set by Redis before any
    // command handler runs, so the `unwrap` cannot fail. `ctx.ctx` is the
    // valid context passed to this command invocation, `event` is a
    // NUL-terminated C string that outlives the call, and `key_name.inner`
    // is a valid `RedisModuleString` owned by `key_name` for the duration
    // of the call.
    let status: Status = unsafe {
        raw::RedisModule_NotifyKeyspaceEvent.unwrap()(
            ctx.ctx,
            NotifyEvent::GENERIC.bits(),
            event.as_ptr(),
            key_name.inner,
        )
    }
    .into();
    match status {
        Status::Ok => Ok("Event sent".into()),
        Status::Err => Err(RedisError::Str("Generic error")),
    }
}

fn last_generic_event(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(LAST_GENERIC_EVENT.lock().unwrap().clone().into())
}

fn on_key_miss(_ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &[u8]) {
    NUM_KEY_MISSES.fetch_add(1, Ordering::SeqCst);
}

fn num_key_miss(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(NUM_KEY_MISSES.load(Ordering::SeqCst)))
}

fn on_new_key(_ctx: &Context, _event_type: NotifyEvent, _event: &str, _key: &[u8]) {
    NUM_KEYS.fetch_add(1, Ordering::SeqCst);
}

fn num_keys(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
    Ok(RedisValue::Integer(NUM_KEYS.load(Ordering::SeqCst)))
}
//////////////////////////////////////////////////////

redis_module! {
    name: "events",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["events.send", event_send, "", 0, 0, 0, ""],
        ["events.send_invalid_utf8", event_send_invalid_utf8, "", 0, 0, 0, ""],
        ["events.last_generic_event", last_generic_event, "", 0, 0, 0, ""],
        ["events.num_key_miss", num_key_miss, "", 0, 0, 0, ""],
        ["events.num_keys", num_keys, "", 0, 0, 0, ""],
    ],
    event_handlers: [
        [@STRING: on_event],
        [@GENERIC: on_generic],
        [@STREAM: on_stream],
        [@MISSED: on_key_miss],
        [@NEW: on_new_key],
    ],
}
