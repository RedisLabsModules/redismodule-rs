use redis_module::{raw, Context};
use std::ffi::{c_char, CStr};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

static LIVE_STRINGS: AtomicUsize = AtomicUsize::new(0);
static FREE_STRING_CONTEXT: AtomicPtr<raw::RedisModuleCtx> = AtomicPtr::new(ptr::null_mut());

struct MockString {
    references: usize,
    tracked: bool,
}

/// # Safety
/// No pointer arguments are dereferenced by this mock.
unsafe extern "C" fn get_server_info(
    _ctx: *mut raw::RedisModuleCtx,
    _section: *const c_char,
) -> *mut raw::RedisModuleServerInfoData {
    Box::into_raw(Box::new(())).cast()
}

/// # Safety
/// `info` must be an allocation returned by [`get_server_info`] that has not been freed.
unsafe extern "C" fn free_server_info(
    _ctx: *mut raw::RedisModuleCtx,
    info: *mut raw::RedisModuleServerInfoData,
) {
    // SAFETY: get_server_info allocated this pointer as a Box<()>; ServerInfo drops it once.
    drop(unsafe { Box::from_raw(info.cast::<()>()) });
}

/// # Safety
/// `field` must be a valid C string. A non-null `ctx` must point to a live bool indicating
/// whether auto-memory is enabled. Calls must not race with other mock API calls.
unsafe extern "C" fn get_field(
    ctx: *mut raw::RedisModuleCtx,
    _info: *mut raw::RedisModuleServerInfoData,
    field: *const c_char,
) -> *mut raw::RedisModuleString {
    // SAFETY: ServerInfo::field supplies a valid, null-terminated CString for this call.
    if unsafe { CStr::from_ptr(field) }.to_bytes() == b"missing" {
        return ptr::null_mut();
    }
    // SAFETY: non-null mock contexts point to a live bool owned by the test below.
    let tracked = !ctx.is_null() && unsafe { *ctx.cast::<bool>() };
    // RedisModule_ServerInfoGetField returns a newly allocated string with one reference.
    LIVE_STRINGS.fetch_add(1, Ordering::SeqCst);
    Box::into_raw(Box::new(MockString {
        references: 1,
        tracked,
    }))
    .cast()
}

/// # Safety
/// `string` must point to a live [`MockString`] returned by [`get_field`], with exclusive access.
unsafe extern "C" fn retain_string(
    ctx: *mut raw::RedisModuleCtx,
    string: *mut raw::RedisModuleString,
) {
    // SAFETY: get_field allocated a MockString, and its allocation is still live.
    let string = unsafe { &mut *string.cast::<MockString>() };
    // Redis detaches tracked strings; otherwise RetainString acquires an additional reference.
    if !ctx.is_null() && string.tracked {
        string.tracked = false;
    } else {
        string.references += 1;
    }
}

/// # Safety
/// `string` must point to a live [`MockString`] with a positive reference count and exclusive access.
unsafe extern "C" fn free_string(
    ctx: *mut raw::RedisModuleCtx,
    string: *mut raw::RedisModuleString,
) {
    FREE_STRING_CONTEXT.store(ctx, Ordering::SeqCst);
    // SAFETY: get_field allocated a MockString, and each owner drops once.
    let value = unsafe { &mut *string.cast::<MockString>() };
    value.references -= 1;
    if !ctx.is_null() {
        value.tracked = false;
    }
    if value.references == 0 {
        // SAFETY: the last reference is gone, so this allocation has no remaining owners.
        drop(unsafe { Box::from_raw(string.cast::<MockString>()) });
        LIVE_STRINGS.fetch_sub(1, Ordering::SeqCst);
    }
}

#[test]
fn server_info_field_owns_its_string_and_preserves_context() {
    // This integration-test binary has one test, so no other tests access these API globals.
    // SAFETY: the mocks match the Redis API signatures and stay installed until all owners drop.
    unsafe {
        raw::RedisModule_GetServerInfo = Some(get_server_info);
        raw::RedisModule_FreeServerInfo = Some(free_server_info);
        raw::RedisModule_ServerInfoGetField = Some(get_field);
        raw::RedisModule_RetainString = Some(retain_string);
        raw::RedisModule_FreeString = Some(free_string);
    }

    let mut auto_memory_disabled = false;
    let mut auto_memory_enabled = true;
    let contexts = [
        ("null context", Context::dummy()),
        (
            "auto-memory disabled",
            Context::new(ptr::from_mut(&mut auto_memory_disabled).cast()),
        ),
        (
            "auto-memory enabled",
            Context::new(ptr::from_mut(&mut auto_memory_enabled).cast()),
        ),
    ];

    for (case, context) in contexts {
        let info = context.server_info("replication");
        assert!(info.field("missing").is_none(), "{case}");
        // Reject embedded NUL instead of panicking or looking up the truncated name "role".
        assert!(info.field("role\0suffix").is_none(), "{case}");
        assert_eq!(LIVE_STRINGS.load(Ordering::SeqCst), 0, "{case}");

        let field = info.field("role").expect("field should exist");
        assert_eq!(LIVE_STRINGS.load(Ordering::SeqCst), 1, "{case}");
        // Derived Debug includes the stored context; preserve its existing observable value.
        assert!(
            format!("{field:?}").contains(&format!("ctx: {:p}", context.ctx)),
            "{case}"
        );

        // The returned string owns its allocation independently of the ServerInfo wrapper.
        drop(info);
        drop(field);

        assert_eq!(LIVE_STRINGS.load(Ordering::SeqCst), 0, "{case}");
        assert_eq!(
            FREE_STRING_CONTEXT.load(Ordering::SeqCst),
            context.ctx,
            "{case}"
        );
    }
}
