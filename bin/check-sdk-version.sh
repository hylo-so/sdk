#!/bin/bash
set -eu pipefail

# Check if there are any changes using cargo workspaces
cargo workspaces changed --error-on-empty >/dev/null 2>&1 || {
  echo "No changes detected in workspaces. Skipping version check."
  exit 0
}

function check-sdk-version() {
  CURRENT_VERSION=$1
  MAIN_VERSION=$2
  TAG=$3

  # Split versions into arrays
  IFS='.' read -ra CURRENT <<< "$CURRENT_VERSION"
  IFS='.' read -ra MAIN <<< "$MAIN_VERSION"

  # Calculate expected version
  EXPECTED_VERSION_PATCHED="${MAIN[0]}.${MAIN[1]}.$((MAIN[2] + 1))"
  EXPECTED_VERSION_MINOR="${MAIN[0]}.$((MAIN[1] + 1)).0"

  # Check if versions have the same major and minor, and patch is exactly one higher
  if [ "$CURRENT_VERSION" = "$EXPECTED_VERSION_PATCHED" ] || [ "$CURRENT_VERSION" = "$EXPECTED_VERSION_MINOR" ]; 
  then
      echo "$3 SDK version check passed"
  else
      echo "Error: $3 SDK version ($CURRENT_VERSION) must be either one patch or full minor version above main branch ($MAIN_VERSION)"
      echo "Please update $3 SDK version to: $EXPECTED_VERSION_PATCHED or $EXPECTED_VERSION_MINOR"
      exit 1
  fi
}

function rust-sdk-version() {
  if [ -n "${1:-}" ]; then
    grep -E -o '[0-9]+\.[0-9]+\.[0-9]+' "$1" | head -1
  else
    grep -E -o '[0-9]+\.[0-9]+\.[0-9]+' | head -1
  fi
}

# Fetch the main branch version
git fetch origin main:main

RUST_SDK=$(rust-sdk-version Cargo.toml)
RUST_SDK_MAIN=$(git show main:Cargo.toml | rust-sdk-version)
check-sdk-version $RUST_SDK $RUST_SDK_MAIN Rust
