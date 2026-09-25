{
  description = "Lunchbox canonical game database and native frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Current Eden AppImages use DwarFS. Keep its NixOS extraction tool on a
    # known-good nixpkgs revision while the primary rolling input advances.
    nixpkgs-dwarfs.url = "github:NixOS/nixpkgs/a5cc6f2c37bf518436dc8d1c288ccd0c43c2f4c4";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-dwarfs, flake-utils }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ] (system:
      let
        pkgs = import nixpkgs { inherit system; };
        onnxruntimeRocm = pkgs.onnxruntime.override { rocmSupport = true; };
        onnxruntimeForHost = if system == "x86_64-linux" then onnxruntimeRocm else pkgs.onnxruntime;
        dwarfsPkgs = import nixpkgs-dwarfs { inherit system; };
        dwarfs = dwarfsPkgs.dwarfs;
        # MAME's CHD core, linked so compressed disc images (CHD) work on every
        # host without a chdman install. libchdman-rs ships its static archives
        # as release assets; the URLs and hashes below are the pinned inputs,
        # and build.rs links whichever one matches the target triple.
        chdmanArchives = {
          x86_64-linux = {
            name = "libchdman_rs-x86_64-unknown-linux-gnu-glibc2.35.a";
            hash = "sha256-U0D05l0P0MQNWx3Txq0dXaplPdt/k6/7cPBIDLx4Ghc=";
          };
          aarch64-linux = {
            name = "libchdman_rs-aarch64-unknown-linux-gnu-glibc2.35.a";
            hash = "sha256-Xh5Y9nl4goWdvHhFFhIOdA4XhTm4GALTY4OJTNNH7Gw=";
          };
          aarch64-darwin = {
            name = "libchdman_rs-aarch64-apple-darwin.a";
            hash = "sha256-vLumhaclyOhG0zS/pt6To+4rHevQUtVWW856UBz+i+g=";
          };
        };
        chdmanArchive = pkgs.fetchurl {
          url = "https://github.com/danifunker/libchdman-rs/releases/download/v0.289.0/${chdmanArchives.${system}.name}";
          hash = chdmanArchives.${system}.hash;
        };
        qtModules = with pkgs.qt6; [
          qtbase
          qtdeclarative
          qtimageformats
          qtmultimedia
          qtquick3d
          qtsvg
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
          qtwayland
          # The desktop Quick Controls style follows KDE's active widget
          # theme and color scheme instead of Qt's generic Linux Fusion style.
          pkgs.kdePackages.qqc2-desktop-style
        ];
        qtEnv = pkgs.qt6.env "lunchbox-qt-env" qtModules;
        databaseTool = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox-db";
          version = "0.1.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          LUNCHBOX_7Z = "${pkgs.p7zip}/bin/7z";
          # Link the pinned MAME CHD archive instead of compiling MAME or
          # downloading an asset inside the sandbox.
          LIBCHDMAN_PREBUILT_LOCAL_ARCHIVE = "${chdmanArchive}";
          cargoBuildFlags = [ "--package" "lunchbox-db" ];
          cargoTestFlags = [ "--package" "lunchbox-db" ];
        };
        controllerProbe = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox-controller-probe";
          version = "0.1.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          cargoBuildFlags = [ "--package" "lunchbox-controller-probe" ];
          cargoTestFlags = [ "--package" "lunchbox-controller-probe" ];
          meta = with pkgs.lib; {
            description = "Target-runtime SDL3 controller inventory for Lunchbox adapters";
            license = licenses.mit;
            mainProgram = "lunchbox-controller-probe";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
        frontend = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox";
          version = "0.1.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            p7zip
            pkg-config
            qt6.wrapQtAppsHook
          ];
          buildInputs = qtModules ++ [ onnxruntimeForHost ]
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            dwarfs
            pkgs.systemd
          ];
          dontUseCmakeConfigure = true;
          dontUseNinjaBuild = true;
          dontUseNinjaInstall = true;
          QMAKE = "${qtEnv}/bin/qmake";
          ORT_LIB_PATH = "${onnxruntimeForHost}/lib";
          ORT_DYLIB_PATH = "${onnxruntimeForHost}/lib/${if pkgs.stdenv.hostPlatform.isDarwin then "libonnxruntime.dylib" else "libonnxruntime.so"}";
          ORT_PREFER_DYNAMIC_LINK = "1";
          LIBCHDMAN_PREBUILT_LOCAL_ARCHIVE = "${chdmanArchive}";
          LUNCHBOX_SDL3_LIBRARY = "${pkgs.lib.getLib pkgs.sdl3}/lib/${if pkgs.stdenv.hostPlatform.isDarwin then "libSDL3.dylib" else "libSDL3.so.0"}";
          # Release builds embed the exact flake revision for the UI build
          # label; the build time is stamped in preBuild.
          LUNCHBOX_BUILD_HASH = self.shortRev or self.dirtyShortRev or "";
          preBuild = ''
            export PATH="${qtEnv}/bin:${qtEnv}/libexec:$PATH"
            export QMAKE="${qtEnv}/bin/qmake"
            export QT_INCLUDE_PATH="${qtEnv}/include"
            export QT_LIBEXEC_PATH="${qtEnv}/libexec"
            # Stamp the real build time into the binary for the window's build
            # label. Nix caches by derivation, so an unchanged input keeps the
            # existing build; a source change produces the new timestamp.
            export LUNCHBOX_BUILT_UNIX="$(date +%s)"
          '';
          cargoBuildFlags = [ "--package" "lunchbox-app" "--package" "lunchbox-controller-probe" "--bin" "lunchbox" "--bin" "lunchbox-controller-probe" ]
            ++ pkgs.lib.optionals (system == "x86_64-linux") [ "--features" "rocm-ocr" ];
          doCheck = true;
          checkPhase = ''
            runHook preCheck
            export QML2_IMPORT_PATH="${qtEnv}/lib/qt-6/qml"
            export QML_IMPORT_PATH="${qtEnv}/lib/qt-6/qml"
            export QT_PLUGIN_PATH="${qtEnv}/lib/qt-6/plugins"
            export XDG_CACHE_HOME="$TMPDIR/lunchbox-test-cache"
            # Display-session tests write private RetroArch appendconfigs via
            # ProjectDirs. A Nix build has no writable user home.
            export XDG_DATA_HOME="$TMPDIR/lunchbox-test-data"
            mkdir -p "$XDG_CACHE_HOME" "$XDG_DATA_HOME"
            QT_QPA_PLATFORM=offscreen qmltestrunner \
              -input crates/lunchbox-app/tests/qml
            cargo test --package lunchbox-app --lib --release \
              ${pkgs.lib.optionalString (system == "x86_64-linux") "--features rocm-ocr"} \
              --target ${pkgs.stdenv.hostPlatform.rust.rustcTarget}
            runHook postCheck
          '';
          postInstall = ''
            mkdir -p "$out/share/lunchbox"
            7z x -y "artifacts/lunchbox.db.7z" "-o$out/share/lunchbox" >/dev/null
            install -Dm644 assets/lunchbox.svg \
              "$out/share/icons/hicolor/scalable/apps/io.github.benwbooth.Lunchbox.svg"
            install -Dm644 packaging/io.github.benwbooth.Lunchbox.desktop \
              "$out/share/applications/io.github.benwbooth.Lunchbox.desktop"
            install -Dm644 packaging/io.github.benwbooth.Lunchbox.metainfo.xml \
              "$out/share/metainfo/io.github.benwbooth.Lunchbox.metainfo.xml"
          '';
          preFixup = ''
            qtWrapperArgs+=(--set LUNCHBOX_DATABASE "$out/share/lunchbox/lunchbox.db")
            qtWrapperArgs+=(--set ORT_DYLIB_PATH "${onnxruntimeForHost}/lib/${if pkgs.stdenv.hostPlatform.isDarwin then "libonnxruntime.dylib" else "libonnxruntime.so"}")
          '' + pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
            qtWrapperArgs+=(--prefix PATH : "${pkgs.lib.makeBinPath [ dwarfs ]}")
          '';
          meta = with pkgs.lib; {
            description = "Native Rust and Qt game library frontend";
            license = licenses.mit;
            mainProgram = "lunchbox";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
        # Iteration builds: identical frontend without the release test
        # suite. `nix run .#lunchbox-fast` launches in a fraction of the
        # time; the default package keeps full gates for CI and commits.
        # The fast profile also drops release LTO/single-codegen (parallel
        # codegen, no thin-LTO link) via pure env overrides.
        frontendFast = frontend.overrideAttrs (previous: {
          pname = "lunchbox-fast";
          doCheck = false;
          CARGO_PROFILE_RELEASE_LTO = "false";
          CARGO_PROFILE_RELEASE_CODEGEN_UNITS = "16";
        });
      in
      {
        checks.controller-probe = controllerProbe;
        packages = {
          default = frontend;
          lunchbox = frontend;
          lunchbox-fast = frontendFast;
          lunchbox-db = databaseTool;
          lunchbox-controller-probe = controllerProbe;
        } // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
          simcoupe-controller = pkgs.callPackage ./packaging/simcoupe-controller.nix { };
          retroarch-relative-routing = pkgs.callPackage ./packaging/retroarch-relative-routing.nix { };
          mame-game-mouse-only = pkgs.callPackage ./packaging/mame-game-mouse-only.nix { };
        } // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
          onnxruntime-rocm = onnxruntimeRocm;
        };

        apps.default = {
          type = "app";
          program = "${frontend}/bin/lunchbox";
        };

        apps.lunchbox-db = {
          type = "app";
          program = "${databaseTool}/bin/lunchbox-db";
        };

        apps.lunchbox-controller-probe = {
          type = "app";
          program = "${controllerProbe}/bin/lunchbox-controller-probe";
        };

        apps.lunchbox-fast = {
          type = "app";
          program = "${frontendFast}/bin/lunchbox";
        };

        devShells.default = pkgs.mkShell {
          LUNCHBOX_SDL3_LIBRARY = "${pkgs.lib.getLib pkgs.sdl3}/lib/${if pkgs.stdenv.hostPlatform.isDarwin then "libSDL3.dylib" else "libSDL3.so.0"}";
          packages = (with pkgs; [
            cargo
            cmake
            clippy
            ninja
            p7zip
            pkg-config
            rust-analyzer
            rustc
            rustfmt
            sqlite
            watchexec
          ]) ++ qtModules ++ [ onnxruntimeForHost ]
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
            dwarfs
            pkgs.mold
            pkgs.systemd
          ];

          QMAKE = "${qtEnv}/bin/qmake";
          ORT_LIB_PATH = "${onnxruntimeForHost}/lib";
          ORT_DYLIB_PATH = "${onnxruntimeForHost}/lib/${if pkgs.stdenv.hostPlatform.isDarwin then "libonnxruntime.dylib" else "libonnxruntime.so"}";
          ORT_PREFER_DYNAMIC_LINK = "1";
          LIBCHDMAN_PREBUILT_LOCAL_ARCHIVE = "${chdmanArchive}";
          QT_QPA_PLATFORM = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux "wayland;xcb";
          # Local dev builds show the real build time; release builds set the
          # pinned flake time instead.
          LUNCHBOX_BUILT_UNIX = "now";

          shellHook = ''
            export PATH="${qtEnv}/bin:${qtEnv}/libexec:$PATH"
            export QMAKE="${qtEnv}/bin/qmake"
            export QT_INCLUDE_PATH="${qtEnv}/include"
            export QT_LIBEXEC_PATH="${qtEnv}/libexec"
            # mold only exists in the dev shell (Linux); nix builds use the
            # default linker so sandboxed derivations stay reproducible.
            ${pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              export RUSTFLAGS="''${RUSTFLAGS:-} -C link-arg=-fuse-ld=mold"
            ''}
            # Incremental dev loop: rebuild the app on any code change and
            # restart it, instead of waiting on a release build or CI. The
            # first debug link takes a few minutes; later one-file Rust
            # rebuilds are around twenty seconds.
            lunchbox-dev() {
              watchexec --restart --shell=none \
                --watch crates \
                --watch vendor \
                --watch Cargo.toml \
                --watch Cargo.lock \
                --exts rs,qml,json,toml,lock \
                -- cargo run -p lunchbox-app --bin lunchbox
            }
            export -f lunchbox-dev 2>/dev/null || true
          '';
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
