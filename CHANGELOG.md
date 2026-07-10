# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.2.0](https://github.com/RedisLabsModules/redismodule-rs/compare/redis-module-v2.0.8...redis-module-v2.2.0) - 2026-07-09

### Added

- Add `KeyFlags::ACCESS_TRIMMED` flag.

### Changed

- `RedisKey::hash_get` accepts any `impl AsRef<[u8]>` as the field argument instead of only `&str` (MOD-16165).

### Other

- Skip versions 2.1.0 through 2.1.3. These exist as git tags on the `2.1` branch and were not published to crates.io. All changes from that branch are included in this release.

## [2.0.8](https://github.com/RedisLabsModules/redismodule-rs/compare/redis-module-v2.0.7...redis-module-v2.0.8) - 2026-05-04

### Fixed

- CI: Revamp the release pipeline, moving over to Trusted Publishing for all crates in the repository.
