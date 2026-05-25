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
        lib = pkgs.lib;
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
          targets = [ "wasm32-unknown-unknown" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
        appId = "io.github.benwbooth.lunchbox";
        version = "0.1.0";
        lunchbox = rustPlatform.buildRustPackage {
          pname = "lunchbox";
          inherit version;

          src = lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = with pkgs; [
            trunk
            wasm-bindgen-cli_0_2_106
            binaryen
            pkg-config
            makeWrapper
          ];

          buildInputs = with pkgs; [
            openssl
            sqlite
            libsecret
            zstd
          ];

          doCheck = false;

          buildPhase = ''
            runHook preBuild
            export HOME="$TMPDIR"
            export XDG_CACHE_HOME="$TMPDIR/.cache"
            cargo build --release -p lunchbox --bin dev_server
            trunk build --release --public-url ./
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall

            install -Dm755 target/release/dev_server "$out/libexec/lunchbox/lunchbox-server"

            mkdir -p "$out/share/lunchbox/frontend"
            cp -r dist/. "$out/share/lunchbox/frontend/"

            mkdir -p "$out/share/lunchbox/electron"
            cp electron/main.cjs electron/package.json "$out/share/lunchbox/electron/"

            mkdir -p "$out/share/lunchbox"
            find db -maxdepth 1 \( -name '*.db' -o -name '*.db.zst' \) -exec cp '{}' "$out/share/lunchbox/" ';'

            install -Dm644 backend/icons/128x128.png "$out/share/icons/hicolor/128x128/apps/${appId}.png"
            install -Dm644 backend/icons/icon.png "$out/share/icons/hicolor/512x512/apps/${appId}.png"
            install -Dm644 packaging/linux/${appId}.desktop "$out/share/applications/${appId}.desktop"
            install -Dm644 packaging/linux/${appId}.metainfo.xml "$out/share/metainfo/${appId}.metainfo.xml"

            makeWrapper ${pkgs.electron}/bin/electron "$out/bin/lunchbox" \
              --add-flags "$out/share/lunchbox/electron/main.cjs" \
              --set LUNCHBOX_RELEASE 1 \
              --set LUNCHBOX_FRONTEND_DIR "$out/share/lunchbox/frontend" \
              --set LUNCHBOX_BACKEND_BIN "$out/libexec/lunchbox/lunchbox-server" \
              --set LUNCHBOX_SHARED_DATA_DIR "$out/share/lunchbox" \
              --set LUNCHBOX_WINDOW_ICON "$out/share/icons/hicolor/512x512/apps/${appId}.png"

            runHook postInstall
          '';

          meta = {
            description = "Cross-platform emulator frontend";
            homepage = "https://github.com/benwbooth/lunchbox";
            license = lib.licenses.mit;
            maintainers = [ ];
            mainProgram = "lunchbox";
            platforms = lib.platforms.linux;
          };
        };
      in
      {
        packages = lib.optionalAttrs pkgs.stdenv.isLinux {
          default = lunchbox;
          lunchbox = lunchbox;
        };

        apps = lib.optionalAttrs pkgs.stdenv.isLinux {
          default = {
            type = "app";
            program = "${lunchbox}/bin/lunchbox";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            trunk
            wasm-bindgen-cli_0_2_106
            binaryen

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

            # Release packaging
            nodejs_22
            flatpak
            flatpak-builder

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

        formatter = pkgs.nixpkgs-fmt;
      });
}
