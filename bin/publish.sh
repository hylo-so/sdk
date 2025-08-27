#!/bin/bash

cargo build --release
cargo publish --package hylo-idl
cargo publish --package hylo-core
cargo publish --package hylo-clients
cargo publish --package hylo-jupiter
