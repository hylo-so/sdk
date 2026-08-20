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
      published_version=$(curl -sf -A hylo-sdk-publish \
        https://crates.io/api/v1/crates/hylo-core \
        | jq -r '.crate.max_version')
      if [ "$local_version" = "$published_version" ]; then
        echo "Version $local_version already on crates.io. Skipping publish."
        exit 0
      fi
      nix develop --command bash -c '
        set -euo pipefail
        cargo build --release
        cargo doc --workspace --no-deps
        cargo publish --package hylo-idl
        cargo publish --package hylo-core
        cargo publish --package hylo-clients
        cargo publish --package hylo-stats
        cargo publish --package hylo-quotes
        cargo publish --package hylo-jupiter
      '
    '';
  };
}
