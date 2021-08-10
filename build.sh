#!/usr/bin/env sh
cargo build --features experimental-api --all --all-targets
cargo build --features test --example test_helper --all-targets


