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
        .header("src/include/redismodule.h")
        .whitelist_var("(REDIS|Redis).*")
        .generate()
        .expect("error generating bindings");

    bindings
        .write_to_file("src/redisraw/bindings.rs")
        .expect("failed to write bindings to file");

    // Do the same trick for RediSearch

    cc::Build::new()
        .file("src/redisearch/redisearch_api.c")
        //.include("src/include/") // For redismodule.h
        .include("src/redisearch/include/")
        .compile("redisearch_api");

    let redisearch_bindings = bindgen::Builder::default()
        .header("src/redisearch/include/redisearch_api.h")
        //.clang_arg("-I src/include") // For redismodule.h
        .whitelist_var("(RS|RediSearch).*")
        .generate()
        .expect("error generating RediSearch bindings");

    redisearch_bindings
        .write_to_file("src/redisearch/raw/bindings.rs")
        .expect("failed to write RediSearch bindings to file");
}
