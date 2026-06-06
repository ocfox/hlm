{
  description = "Hyperliquid price monitor overlay";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{ flake-parts, crane, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      perSystem =
        { pkgs, system, ... }:
        let
          craneLib = crane.mkLib pkgs;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            libxkbcommon
            wayland
          ];

          commonArgs = {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;
            inherit nativeBuildInputs buildInputs;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          hlm = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
            }
          );
        in
        {
          packages.default = hlm;

          devShells.default = pkgs.mkShell {
            inputsFrom = [ hlm ];
            packages = with pkgs; [
              cargo
              rustc
              rust-analyzer
              clippy
            ];
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (
              with pkgs;
              [
                wayland
                libxkbcommon
              ]
            );
          };
        };
    };
}
