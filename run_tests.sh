#!/usr/bin/bash

echo "Testing lts v0.70"
cargo test --no-default-features --features fuels_lts_70 -- --test-threads 1

echo "Testing v0.71"
cargo test --no-default-features --features fuels_71 -- --test-threads 1
