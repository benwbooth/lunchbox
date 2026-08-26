{
  description = "Lunchbox canonical game database and native frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        databaseTool = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox-db";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          LUNCHBOX_7Z = "${pkgs.p7zip}/bin/7z";
        };
      in
      {
        packages = {
          default = databaseTool;
          lunchbox-db = databaseTool;
        };

        apps.default = {
          type = "app";
          program = "${databaseTool}/bin/lunchbox-db";
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            clippy
            p7zip
            rust-analyzer
            rustc
            rustfmt
            sqlite
          ];
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
