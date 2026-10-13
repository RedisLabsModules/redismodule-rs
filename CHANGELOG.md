# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `RedisKey::hash_get_by_string` and `RedisKeyWritable::hash_get_by_string`: `HGET` with the field passed as an existing `RedisString`, avoiding the per-call `CString` and the transient field object Redis creates for C-string fields.

## [2.0.8](https://github.com/RedisLabsModules/redismodule-rs/compare/redis-module-v2.0.7...redis-module-v2.0.8) - 2026-05-04

### Fixed

- CI: Revamp the release pipeline, moving over to Trusted Publishing for all crates in the repository.
