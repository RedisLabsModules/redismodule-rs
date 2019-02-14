[![GitHub issues](https://img.shields.io/github/release/RedisLabsModules/redismodule-rs.svg)](https://github.com/RedisLabsModules/redismodule-rs/releases/latest)
[![CircleCI](https://circleci.com/gh/RedisLabsModules/redismodule-rs/tree/master.svg?style=svg)](https://circleci.com/gh/RedisLabsModules/redismodule-rs/tree/master)

# redismodule-rs

The Rust API for Redis is based on the https://github.com/brandur/redis-cell project.

# Running the example module
1. install [rust and cargo](https://www.rust-lang.org/tools/install) 
2. install redis [(mac)](https://gist.github.com/tomysmile/1b8a321e7c58499ef9f9441b2faa0aa8)
3. run `cargo build --example hello`
4. start a redis server with the hello module 
`redis-server --loadmodule ./target/debug/examples/libhello.dylib`
5. open a redis cli. and run `HELLO.ADD 31 11`. 
