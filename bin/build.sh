#!/bin/bash
set -eu pipefail

cargo build
cargo test --workspace --exclude hylo-jupiter
cargo test --doc
