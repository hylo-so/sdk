#!/bin/bash

cargo build --release
cargo doc --workspace --no-deps
cargo publish --package hylo-idl
cargo publish --package hylo-core
cargo publish --package hylo-clients
cargo publish --package hylo-jupiter
