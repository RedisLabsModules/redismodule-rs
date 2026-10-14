use redis_module::{raw, Context};
use std::cell::Cell;
use std::ffi::{c_char, CStr};
use std::ptr::{self, NonNull};
use std::sync::Once;

static INIT: Once = Once::new();
thread_local! {
    static REFERENCES: Cell<usize> = const { Cell::new(0) };
    static FIELD_CALLS: Cell<usize> = const { Cell::new(0) };
    static NON_NULL_STRING_CONTEXT: Cell<bool> = const { Cell::new(false) };
}

extern "C" fn get_info(
    _: *mut raw::RedisModuleCtx,
    _: *const c_char,
) -> *mut raw::RedisModuleServerInfoData {
    NonNull::dangling().as_ptr()
}

extern "C" fn free_info(_: *mut raw::RedisModuleCtx, _: *mut raw::RedisModuleServerInfoData) {}

/// # Safety
/// The field name must point to a valid, null-terminated C string during this call.
unsafe extern "C" fn get_field(
    ctx: *mut raw::RedisModuleCtx,
    _: *mut raw::RedisModuleServerInfoData,
    field: *const c_char,
) -> *mut raw::RedisModuleString {
    FIELD_CALLS.set(FIELD_CALLS.get() + 1);
    NON_NULL_STRING_CONTEXT.set(NON_NULL_STRING_CONTEXT.get() || !ctx.is_null());
    // SAFETY: ServerInfo::field supplies a live CString for this call.
    if unsafe { CStr::from_ptr(field) }.to_bytes() == b"missing" {
        return ptr::null_mut();
    }
    REFERENCES.set(1);
    NonNull::dangling().as_ptr()
}

extern "C" fn retain(_: *mut raw::RedisModuleCtx, _: *mut raw::RedisModuleString) {
    REFERENCES.set(REFERENCES.get() + 1);
}

extern "C" fn free(ctx: *mut raw::RedisModuleCtx, _: *mut raw::RedisModuleString) {
    NON_NULL_STRING_CONTEXT.set(NON_NULL_STRING_CONTEXT.get() || !ctx.is_null());
    REFERENCES.set(REFERENCES.get() - 1);
}

fn context() -> Context {
    INIT.call_once(|| {
        // SAFETY: mocks match the API signatures, never dereference opaque handles, and are
        // installed once before use. Each test thread has independent reference-count state.
        unsafe {
            raw::RedisModule_GetServerInfo = Some(get_info);
            raw::RedisModule_FreeServerInfo = Some(free_info);
            raw::RedisModule_ServerInfoGetField = Some(get_field);
            raw::RedisModule_RetainString = Some(retain);
            raw::RedisModule_FreeString = Some(free);
        }
    });
    REFERENCES.set(0);
    FIELD_CALLS.set(0);
    NON_NULL_STRING_CONTEXT.set(false);
    // An opaque non-null handle catches accidentally retaining the caller's context.
    Context::new(NonNull::dangling().as_ptr())
}

#[test]
fn field_owns_one_reference_independently_of_server_info_and_context() {
    let info = context().server_info("replication");
    let field = info.field("role").expect("field should exist");
    assert_eq!(REFERENCES.get(), 1);
    drop(info);
    assert_eq!(REFERENCES.get(), 1);
    drop(field);
    assert_eq!(REFERENCES.get(), 0);
    assert!(!NON_NULL_STRING_CONTEXT.get());
}

#[test]
fn missing_field_returns_none() {
    let info = context().server_info("replication");
    assert!(info.field("missing").is_none());
    assert_eq!(REFERENCES.get(), 0);
}

#[test]
fn embedded_nul_returns_none_without_calling_redis() {
    let info = context().server_info("replication");
    assert!(info.field("role\0suffix").is_none());
    assert_eq!(FIELD_CALLS.get(), 0);
    assert_eq!(REFERENCES.get(), 0);
}
