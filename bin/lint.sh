#!/bin/bash
set -eu pipefail

cargo-fmt --check
cargo-clippy --check -- --deny clippy::pedantic
