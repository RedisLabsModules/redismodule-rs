extern crate bindgen;
extern crate cc;

fn main() {
    // Build a Redis pseudo-library so that we have symbols that we can link
    // against while building Rust code.
    //
    // include/redismodule.h is vendored in from the Redis project and
    // src/redismodule.c is a stub that includes it and plays a few other
    // tricks that we need to complete the build.

    let mut build = cc::Build::new();

    // if the `experimental-api` is enabled
    if std::env::var_os("CARGO_FEATURE_EXPERIMENTAL_API").is_some() {
        build.define("REDISMODULE_EXPERIMENTAL_API", None);
    }

    build
        .file("src/redismodule.c")
        .include("src/include/")
        .compile("redismodule");

    let mut build = bindgen::Builder::default();

    // if the `experimental-api` is enabled
    if std::env::var_os("CARGO_FEATURE_EXPERIMENTAL_API").is_some() {
        build = build.clang_arg("-DREDISMODULE_EXPERIMENTAL_API");
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
