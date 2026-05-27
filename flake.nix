{
  description = "Solana dev environment for Hylo protocol";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };
  outputs = inputs@{ self, nixpkgs, flake-parts, rust-overlay }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems =
        [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
      perSystem = { config, self', inputs', pkgs, system, ... }:
        with import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        let
          sharedBuildInputs = [ libiconv pkg-config gcc openssl ];
          rustStable = rust-bin.stable."1.88.0".default.override {
            extensions = [ "rust-analyzer" "rust-src" ];
          };
          shellTools =
            import ./shell-tools.nix { inherit writeShellApplication; };
        in {
          devShells.nightly = mkShell {
            packages = [ rust-bin.nightly.latest.default cargo-udeps ]
              ++ builtins.attrValues shellTools;
            buildInputs = sharedBuildInputs;
          };

          devShells.default = mkShell {
            packages = [ rustStable cargo-workspaces ]
              ++ builtins.attrValues shellTools;
            buildInputs = sharedBuildInputs;
          };

          devShells.kani = mkShell {
            packages = [ rustup cmake ];
            buildInputs = sharedBuildInputs;
          };

          packages = shellTools;
        };
    };
}
