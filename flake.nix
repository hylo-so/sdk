{
  description = "Solana dev environment for Hylo protocol";
  inputs = {
    nixpkgs.url =
      "github:NixOS/nixpkgs/27fd171e9a51865e9b445c5583d1a1e06235efb2";
    flake-parts.url =
      "github:hercules-ci/flake-parts/9126214d0a59633752a136528f5f3b9aa8565b7d";
    rust-overlay.url =
      "github:oxalica/rust-overlay/9127ca1f5a785b23a2fc1c74551a27d3e8b9a28b";
  };
  outputs = inputs@{ self, nixpkgs, flake-parts, rust-overlay }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems =
        [ "aarch64-darwin" "aarch64-linux" "x86_64-darwin" "x86_64-linux" ];
      perSystem = { config, self', inputs', pkgs, system, ... }:
        with import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ]; }; let
          sharedBuildInputs = [ libiconv pkg-config gcc openssl ]
            ++ lib.optionals stdenv.isDarwin
            (with darwin.apple_sdk.frameworks; [
              System
              Security
              SystemConfiguration
              CoreFoundation
              CoreServices
              Foundation
            ]);
        in {
          devShells.nightly = mkShell {
            packages =
              [ rust-bin.nightly.latest.default ];
            buildInputs = sharedBuildInputs;
          };
          devShells.default = mkShell {
            packages = [ rust-bin.stable."1.88.0".default cargo-workspaces ]
              ++ lib.optionals stdenv.isDarwin [ rust-analyzer ];
            buildInputs = sharedBuildInputs;
          };
        };
    };
}
