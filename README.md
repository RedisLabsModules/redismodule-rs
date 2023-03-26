[![license](https://img.shields.io/github/license/RedisLabsModules/redismodule-rs.svg)](https://github.com/RedisLabsModules/redismodule-rs/blob/master/LICENSE)
[![GitHub issues](https://img.shields.io/github/release/RedisLabsModules/redismodule-rs.svg)](https://github.com/RedisLabsModules/redismodule-rs/releases/latest)
[![CircleCI](https://circleci.com/gh/RedisLabsModules/redismodule-rs/tree/master.svg?style=svg)](https://circleci.com/gh/RedisLabsModules/redismodule-rs/tree/master)
[![crates.io](https://img.shields.io/crates/v/redis-module.svg)](https://crates.io/crates/redis-module)

# redismodule-rs

This crate provides an idiomatic Rust API for the [Redis Modules API](https://redis.io/topics/modules-intro).
It allows writing Redis modules in Rust, without needing to use raw pointers or unsafe code.

# Running the example module

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. [Install Redis](https://redis.io/download), most likely using your favorite package manager (Homebrew on Mac, APT or YUM on Linux)
3. Run `cargo build --example hello`
4. Start a redis server with the `hello` module
   * Linux: `redis-server --loadmodule ./target/debug/examples/libhello.so`
   * Mac: `redis-server --loadmodule ./target/debug/examples/libhello.dylib`
5. Open a Redis CLI, and run `HELLO.MUL 31 11`.

# Writing your own module

See the [examples](examples) directory for some sample modules.

This crate tries to provide high-level wrappers around the standard Redis Modules API, while preserving the API's basic concepts.
Therefore, following the [Redis Modules API](https://redis.io/topics/modules-intro) documentation will be mostly relevant here as well.

# Redis Modules based on this crate

The following are some modules that are built on this crate:

- [RedisJSON](https://github.com/RedisJSON/RedisJSON)
- [Rust API for RediSearch](https://github.com/RediSearch/redisearch-api-rs)
- [RedisStateMachine](https://github.com/RedisLabsModules/redis-state-machine)

Several member of the community have written their own modules based on this.
