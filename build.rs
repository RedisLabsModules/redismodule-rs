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
            || name == "REDISMODULE_LIST_HEAD"
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
    //
    // Users can provide a custom redismodule.h by setting the
    // REDIS_MODULE_H environment variable to the path of their custom header file.

    const EXPERIMENTAL_API: &str = "REDISMODULE_EXPERIMENTAL_API";

    // Determine which redismodule.h to use
    let (header_path, include_dir) = if let Ok(custom_header) = env::var("REDIS_MODULE_H") {
        let custom_path = PathBuf::from(&custom_header);

        // Validate that the custom header exists
        if !custom_path.exists() {
            panic!(
                "Custom REDIS_MODULE_H path does not exist: {}",
                custom_header
            );
        }

        // Get the directory containing the custom header for the include path
        let include_dir = custom_path
            .parent()
            .expect("REDIS_MODULE_H must have a parent directory")
            .to_path_buf();

        println!("cargo:warning=Using custom redismodule.h from: {}", custom_header);
        println!("cargo:rerun-if-changed={}", custom_header);

        (custom_path, include_dir)
    } else {
        // Use the default vendored header
        let default_header = PathBuf::from("src/include/redismodule.h");
        let default_include = PathBuf::from("src/include/");

        println!("cargo:rerun-if-changed=src/include/redismodule.h");

        (default_header, default_include)
    };

    let mut build = cc::Build::new();

    build
        .define(EXPERIMENTAL_API, None)
        .file("src/redismodule.c")
        .include(&include_dir)
        .compile("redismodule");

    let bindings_generator = bindgen::Builder::default();

    let bindings = bindings_generator
        .clang_arg(format!("-D{EXPERIMENTAL_API}"))
        .clang_arg(format!("-I{}", include_dir.display()))
        .header(header_path.to_str().expect("Invalid header path"))
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
