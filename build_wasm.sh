#!/bin/sh

set -e
RUSTFLAGS=--cfg=web_sys_unstable_apis cargo build --no-default-features --target wasm32-unknown-unknown
wasm-bindgen --out-dir target/generated --web target/wasm32-unknown-unknown/debug/linon.wasm
