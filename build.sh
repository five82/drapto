#! /usr/bin/env bash

cargo clean
cargo build
cargo test -p drapto-core
cargo test -p drapto-cli
