extern crate bindgen;
extern crate cc;

fn main() {
    // Build a Redis pseudo-library so that we have symbols that we can link
    // against while building Rust code.
    //
    // include/redismodule.h is vendored in from the Redis project and
    // src/redismodule.c is a stub that includes it and plays a few other
    // tricks that we need to complete the build.

    cc::Build::new()
        .file("src/redismodule.c")
        .include("src/include/")
        .compile("redismodule");

    let bindings = bindgen::Builder::default()
        .clang_arg("-DREDISMODULE_EXPERIMENTAL_API")
        .header("src/include/redismodule.h")
        .whitelist_var("(REDIS|Redis).*")
        .generate()
        .expect("error generating bindings");

    bindings
        .write_to_file("src/redisraw/bindings.rs")
        .expect("failed to write bindings to file");
}
