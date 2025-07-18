{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      rust-overlay,
      nixpkgs,
    }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      rust-bin = rust-overlay.lib.mkRustBin { } pkgs.buildPackages;
    in
    {
      devShells.${system}.default =
        pkgs.callPackage (
          {
            mkShell,
            pkg-config,
            stdenv,
          }:
          mkShell {
            nativeBuildInputs = with pkgs; [
              (rust-bin.stable.latest.minimal.override {
                extensions = [ "rustfmt" "rust-src" "rust-analyzer" "clippy" ];
              })
              gcc
              flatbuffers
            ];

            depsBuildBuild = [];
            buildInputs = [];
          }
        ) { };
    };
}
