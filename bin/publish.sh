#!/bin/bash

cargo build --release --package hylo-sdk
cargo publish --package hylo-sdk
