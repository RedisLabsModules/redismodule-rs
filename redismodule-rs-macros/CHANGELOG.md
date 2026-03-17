# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.8](https://github.com/RedisLabsModules/redismodule-rs/compare/redis-module-macros-v2.0.7...redis-module-macros-v2.0.8) - 2026-03-17

### Other

- Revamp the CI pipeline, including publishing ([#428](https://github.com/RedisLabsModules/redismodule-rs/pull/428))
- Fix clippy lints ([#456](https://github.com/RedisLabsModules/redismodule-rs/pull/456))
- MOD-8720 command proc macro: add acl and command arguments parameters
- Address the formatting
- Added support for defrag API. ([#387](https://github.com/RedisLabsModules/redismodule-rs/pull/387))
- Added support for _proxy-filtered flag. ([#366](https://github.com/RedisLabsModules/redismodule-rs/pull/366))
- Refactor the supported maps list again
- Revert the previous info argument of redis_module! macro.
- Provide more information on the compatible structs for the InfoSection derive.
- Refactor the supported maps list
- Rename InfoContext::build_from to build_one_section.
- Revert to the old style of RedisResult.
- Improves the info handler API.
- Added cron server event. ([#349](https://github.com/RedisLabsModules/redismodule-rs/pull/349))
- Added config changes server event ([#343](https://github.com/RedisLabsModules/redismodule-rs/pull/343))
- Extend `RedisValue` derive macro. ([#338](https://github.com/RedisLabsModules/redismodule-rs/pull/338))
- Implements `RedisValue` derive proc macro. ([#335](https://github.com/RedisLabsModules/redismodule-rs/pull/335))
- The PR introduce a new proc macro for command registrations. ([#326](https://github.com/RedisLabsModules/redismodule-rs/pull/326))
- Post notification API and API versioning ([#304](https://github.com/RedisLabsModules/redismodule-rs/pull/304))
