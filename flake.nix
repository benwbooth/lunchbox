{
  description = "Lunchbox - Cross-platform emulator frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            trunk
            wasm-bindgen-cli

            # Build dependencies
            pkg-config
            openssl
            sqlite

            # Additional deps
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good

            # Dev tools
            sqlx-cli
            just
            tailwindcss
            watchexec

            # Browser / desktop shells for development
            chromium
            electron

            # Credential storage (secret-tool CLI)
            libsecret

            # Compression
            zstd
            p7zip
            gnutar
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # Keep flake inputs in closure so GC doesn't collect them
          FLAKE_INPUTS = builtins.concatStringsSep ":" [ "${nixpkgs}" "${rust-overlay}" "${flake-utils}" ];

          # Development shell environment
          shellHook = ''
            export RUST_BACKTRACE=1
          '';
        };
      });
}
