#!/bin/bash
set -eu pipefail

cargo-fmt
cargo-clippy --fix -- --deny clippy::pedantic
