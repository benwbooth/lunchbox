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
            cargo-tauri
            trunk
            wasm-bindgen-cli

            # Build dependencies
            pkg-config
            openssl
            sqlite

            # Tauri dependencies (Linux/GTK)
            webkitgtk_4_1
            gtk3
            libsoup_3
            glib
            gdk-pixbuf
            pango
            cairo
            atk
            librsvg

            # Additional deps
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good

            # WebKitGTK rendering fix for NixOS
            gsettings-desktop-schemas
            glib-networking

            # Dev tools
            sqlx-cli
            just
            nodePackages.tailwindcss

            # Compression
            zstd
          ];

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          # Environment variables for Tauri
          shellHook = ''
            export RUST_BACKTRACE=1
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath [
              pkgs.webkitgtk_4_1
              pkgs.gtk3
              pkgs.libsoup_3
              pkgs.glib
              pkgs.gdk-pixbuf
              pkgs.pango
              pkgs.cairo
              pkgs.atk
              pkgs.librsvg
              pkgs.gst_all_1.gstreamer
              pkgs.gst_all_1.gst-plugins-base
            ]}:$LD_LIBRARY_PATH"
            # Fix WebKitGTK rendering on NixOS (https://github.com/tauri-apps/tauri/issues/14187)
            export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}:$XDG_DATA_DIRS"
            export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
          '';
        };
      });
}
