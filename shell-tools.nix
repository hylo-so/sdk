{ writeShellApplication, curl, jq, gnused }: {
  lint = writeShellApplication {
    name = "lint";
    text = ''
      nix develop .#nightly --command bash -c "
        set -euo pipefail
        cargo-fmt --check
        cargo-clippy --check
      "
    '';
  };

  polish = writeShellApplication {
    name = "polish";
    text = ''
      nix develop .#nightly --command bash -c "
        set -euo pipefail
        cargo-fmt
        cargo-clippy --fix
      "
    '';
  };

  build = writeShellApplication {
    name = "build";
    text = "nix develop --command cargo build";
  };

  verify = writeShellApplication {
    name = "verify";
    text = ''
      nix develop .#kani --command cargo kani "$@"
    '';
  };

  test-cargo = writeShellApplication {
    name = "test-cargo";
    text = ''
      nix develop --command bash -c "
        set -euo pipefail
        cargo test --workspace --exclude hylo-jupiter
        cargo test --workspace --exclude hylo-jupiter --features shadow
        cargo test --doc
      "
    '';
  };

  publish = writeShellApplication {
    name = "publish";
    runtimeInputs = [ curl jq gnused ];
    text = ''
      local_version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml)
      missing=()
      for crate in hylo-idl hylo-core hylo-clients hylo-stats \
        hylo-quotes hylo-jupiter; do
        published=$(curl -sf -A hylo-sdk-publish \
          "https://crates.io/api/v1/crates/$crate" \
          | jq -r '.crate.max_version')
        if [ "$local_version" = "$published" ]; then
          echo "$crate $local_version already on crates.io. Skipping."
        else
          missing+=("$crate")
        fi
      done
      if [ "''${#missing[@]}" -eq 0 ]; then
        echo "All crates at $local_version. Nothing to publish."
        exit 0
      fi
      # shellcheck disable=SC2016
      nix develop --command bash -c '
        set -euo pipefail
        cargo build --release
        cargo doc --workspace --no-deps
        for crate in "$@"; do
          cargo publish --package "$crate"
        done
      ' publish-missing "''${missing[@]}"
    '';
  };
}
