{ writeShellApplication }:
{
  lint = writeShellApplication {
    name = "lint";
    text = ''
      nix develop .#nightly --command bash -c "
        set -euo pipefail
        cargo-fmt --check
        cargo-clippy --check -- --deny clippy::pedantic
      "
    '';
  };

  polish = writeShellApplication {
    name = "polish";
    text = ''
      nix develop .#nightly --command bash -c "
        set -euo pipefail
        cargo-fmt
        cargo-clippy --fix -- --deny clippy::pedantic
      "
    '';
  };

  build = writeShellApplication {
    name = "build";
    text = ''nix develop --command cargo build'';
  };

  test = writeShellApplication {
    name = "test";
    text = ''
      nix develop --command bash -c "
        set -euo pipefail
        cargo test --workspace --exclude hylo-jupiter
        cargo test --doc
      "
    '';
  };

  publish = writeShellApplication {
    name = "publish";
    text = ''
      nix develop --command bash -c '
        set -euo pipefail
        if ! cargo workspaces changed --error-on-empty >/dev/null 2>&1; then
          echo "No changes detected. Skipping publish."
          exit 0
        fi
        cargo build --release
        cargo doc --workspace --no-deps
        cargo publish --package hylo-idl
        cargo publish --package hylo-core
        cargo publish --package hylo-clients
        cargo publish --package hylo-quotes
        cargo publish --package hylo-jupiter
      '
    '';
  };
}
