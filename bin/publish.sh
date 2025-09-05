#!/bin/bash

cargo build --release
cargo doc --workspace --no-deps
cargo workspaces publish --yes
