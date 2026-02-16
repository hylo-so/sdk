{
  description = "Solana dev environment for Hylo protocol";
  inputs = {
    nixpkgs.url =
      "github:NixOS/nixpkgs/36da836b1cd9fcf51b1dacd1f4ba39163649ed13";
    flake-parts.url =
      "github:hercules-ci/flake-parts/9126214d0a59633752a136528f5f3b9aa8565b7d";
    rust-overlay.url =
      "github:oxalica/rust-overlay/42ec85352e419e601775c57256a52f6d48a39906";
    rsmap.url =
      "github:zfedoran/rsmap/941b75c4b1fdd70f433ad755f42cd35c65f9ac61";
  };
  outputs = inputs@{ self, nixpkgs, flake-parts, rust-overlay, rsmap }:
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
            packages =
              [ rustStable cargo-workspaces rsmap.packages.${system}.rsmap ]
              ++ builtins.attrValues shellTools;
            buildInputs = sharedBuildInputs;
          };

          packages = shellTools;
        };
    };
}
