#!/bin/bash
set -eu

# Check if there are any changes using cargo workspaces
if ! cargo workspaces changed --error-on-empty >/dev/null 2>&1; then
  echo "No changes detected in workspaces. Skipping publish."
  exit 0
fi

cargo build --release
cargo doc --workspace --no-deps
cargo publish --package hylo-idl || true
cargo publish --package hylo-core || true
cargo publish --package hylo-clients || true
cargo publish --package hylo-jupiter || true
