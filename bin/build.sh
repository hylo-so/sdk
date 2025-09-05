#!/bin/bash
set -eu pipefail

cargo build
cargo test
cargo test --doc
