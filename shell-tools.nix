{ writeShellApplication }: {
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
    text = ''
      # shellcheck disable=SC2016
      nix develop --command bash -c '
        set -euo pipefail
        if ! cargo workspaces changed --error-on-empty >/dev/null 2>&1; then
          echo "No changes detected. Skipping publish."
          exit 0
        fi
        cargo build --release
        cargo doc --workspace --no-deps
        for pkg in hylo-idl hylo-core hylo-clients hylo-stats hylo-quotes hylo-jupiter; do
          out=$(cargo publish --package "$pkg" 2>&1) && ok=0 || ok=1
          echo "$out"
          if [ "$ok" -eq 1 ]; then
            echo "$out" | grep -q "already exists on crates.io index" || exit 1
            echo "Skipping $pkg: version already published"
          fi
        done
      '
    '';
  };
}
