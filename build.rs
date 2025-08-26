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
        } else if name.starts_with("REDISMODULE_REPLY") {
            Some(IntKind::I32)
        } else if name == "REDISMODULE_LIST_HEAD"
            || name == "REDISMODULE_LIST_TAIL"
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

    let mut build = cc::Build::new();

    build
        .define(EXPERIMENTAL_API, None)
        .file("src/redismodule.c")
        .include("src/include/")
        .compile("redismodule");

    let bindings_generator = bindgen::Builder::default();

    let bindings = bindings_generator
        .clang_arg(format!("-D{EXPERIMENTAL_API}"))
        .header("src/include/redismodule.h")
        .allowlist_var("(REDIS|Redis).*")
        .blocklist_type("__darwin_.*")
        .allowlist_type("RedisModule.*")
        .parse_callbacks(Box::new(RedisModuleCallback))
        .size_t_is_usize(true)
        .generate()
        .expect("error generating bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("failed to write bindings to file");
}
