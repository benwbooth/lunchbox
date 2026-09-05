{
  description = "Lunchbox canonical game database and native frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Current Eden AppImages use DwarFS. Keep its NixOS extraction tool on a
    # known-good nixpkgs revision while the primary rolling input advances.
    nixpkgs-dwarfs.url = "github:NixOS/nixpkgs/a5cc6f2c37bf518436dc8d1c288ccd0c43c2f4c4";
    # Keep Intel macOS on the supported Darwin branch while Apple Silicon and
    # Linux follow unstable.
    nixpkgs-intel-darwin.url = "github:NixOS/nixpkgs/nixpkgs-26.05-darwin";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-dwarfs, nixpkgs-intel-darwin, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        nixpkgsForSystem = if system == "x86_64-darwin" then nixpkgs-intel-darwin else nixpkgs;
        pkgs = import nixpkgsForSystem { inherit system; };
        dwarfsPkgs = import nixpkgs-dwarfs { inherit system; };
        dwarfs = dwarfsPkgs.dwarfs;
        qtModules = with pkgs.qt6; [
          qtbase
          qtdeclarative
          qtimageformats
          qtmultimedia
          qtquick3d
          qtsvg
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ qtwayland ];
        qtEnv = pkgs.qt6.env "lunchbox-qt-env" qtModules;
        databaseTool = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox-db";
          version = "0.1.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          LUNCHBOX_7Z = "${pkgs.p7zip}/bin/7z";
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
          buildInputs = qtModules
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              dwarfs
              pkgs.systemd
            ];
          dontUseCmakeConfigure = true;
          dontUseNinjaBuild = true;
          dontUseNinjaInstall = true;
          QMAKE = "${qtEnv}/bin/qmake";
          preBuild = ''
            export PATH="${qtEnv}/bin:${qtEnv}/libexec:$PATH"
            export QMAKE="${qtEnv}/bin/qmake"
            export QT_INCLUDE_PATH="${qtEnv}/include"
            export QT_LIBEXEC_PATH="${qtEnv}/libexec"
          '';
          cargoBuildFlags = [ "--package" "lunchbox-app" ];
          doCheck = true;
          checkPhase = ''
            runHook preCheck
            export QML2_IMPORT_PATH="${qtEnv}/lib/qt-6/qml"
            export QML_IMPORT_PATH="${qtEnv}/lib/qt-6/qml"
            export QT_PLUGIN_PATH="${qtEnv}/lib/qt-6/plugins"
            export XDG_CACHE_HOME="$TMPDIR/lunchbox-test-cache"
            mkdir -p "$XDG_CACHE_HOME"
            QT_QPA_PLATFORM=offscreen qmltestrunner \
              -input crates/lunchbox-app/tests/qml
            cargo test --package lunchbox-app --lib --release \
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
      in
      {
        checks.controller-probe = controllerProbe;
        packages = {
          default = frontend;
          lunchbox = frontend;
          lunchbox-db = databaseTool;
          lunchbox-controller-probe = controllerProbe;
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

        devShells.default = pkgs.mkShell {
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
          ]) ++ qtModules
            ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              dwarfs
              pkgs.systemd
            ];

          QMAKE = "${qtEnv}/bin/qmake";
          QT_QPA_PLATFORM = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux "wayland;xcb";

          shellHook = ''
            export PATH="${qtEnv}/bin:${qtEnv}/libexec:$PATH"
            export QMAKE="${qtEnv}/bin/qmake"
            export QT_INCLUDE_PATH="${qtEnv}/include"
            export QT_LIBEXEC_PATH="${qtEnv}/libexec"
          '';
        };

        formatter = pkgs.nixpkgs-fmt;
      });
}
