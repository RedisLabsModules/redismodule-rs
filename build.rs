extern crate bindgen;
extern crate cc;

use bindgen::callbacks::{IntKind, ParseCallbacks};
use std::env;
use std::path::PathBuf;

#[derive(Debug)]
struct RedisModuleCallback;

impl ParseCallbacks for RedisModuleCallback {
    fn int_macro(&self, name: &str, _value: i64) -> Option<IntKind> {
        if name.starts_with("REDISMODULE_SUBEVENT_") || name.starts_with("REDISMODULE_EVENT_") {
            Some(IntKind::U64)
        } else if name.starts_with("REDISMODULE_REPLY_")
            || name.starts_with("REDISMODULE_KEYTYPE_")
            || name.starts_with("REDISMODULE_AUX_")
            || name == "REDISMODULE_OK"
            || name == "REDISMODULE_ERR"
        {
            // These values are used as `enum` discriminants, and thus must be `isize`.
            Some(IntKind::Custom {
                name: "isize",
                is_signed: true,
            })
        } else if name.starts_with("REDISMODULE_NOTIFY_") {
            Some(IntKind::Int)
        } else {
            None
        }
    }
}

fn main() {
    // Build a Redis pseudo-library so that we have symbols that we can link
    // against while building Rust code.
    //
    // include/redismodule.h is vendored in from the Redis project and
    // src/redismodule.c is a stub that includes it and plays a few other
    // tricks that we need to complete the build.

    const EXPERIMENTAL_API: &str = "REDISMODULE_EXPERIMENTAL_API";

    // Determine if the `experimental-api` feature is enabled
    fn experimental_api() -> bool {
        std::env::var_os("CARGO_FEATURE_EXPERIMENTAL_API").is_some()
    }

    let mut build = cc::Build::new();

    if experimental_api() {
        build.define(EXPERIMENTAL_API, None);
    }

    build
        .file("src/redismodule.c")
        .include("src/include/")
        .compile("redismodule");

    let mut build = bindgen::Builder::default();

    if experimental_api() {
        build = build.clang_arg(format!("-D{}", EXPERIMENTAL_API).as_str());
    }

    let bindings = build
        .header("src/include/redismodule.h")
        .whitelist_var("(REDIS|Redis).*")
        .blacklist_type("__darwin_.*")
        .whitelist_type("RedisModule.*")
        .parse_callbacks(Box::new(RedisModuleCallback))
        .size_t_is_usize(true)
        .generate()
        .expect("error generating bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings to file");
}
