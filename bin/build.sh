#!/bin/bash
set -eu pipefail

cargo build
cargo test --workspace
cargo test --doc
