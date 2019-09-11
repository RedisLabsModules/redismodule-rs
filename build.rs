extern crate bindgen;
extern crate cc;

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
        .generate()
        .expect("error generating bindings");

    bindings
        .write_to_file("src/redisraw/bindings.rs")
        .expect("failed to write bindings to file");
}
