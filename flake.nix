{
  description = "Lunchbox canonical game database and native frontend";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    # Keep Intel macOS on the supported Darwin branch while Apple Silicon and
    # Linux follow unstable.
    nixpkgs-intel-darwin.url = "github:NixOS/nixpkgs/nixpkgs-26.05-darwin";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, nixpkgs-intel-darwin, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        nixpkgsForSystem = if system == "x86_64-darwin" then nixpkgs-intel-darwin else nixpkgs;
        pkgs = import nixpkgsForSystem { inherit system; };
        qtModules = with pkgs.qt6; [
          qtbase
          qtdeclarative
          qtimageformats
          qtsvg
        ] ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [ qtwayland ];
        qtEnv = pkgs.qt6.env "lunchbox-qt-env" qtModules;
        databaseTool = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox-db";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          LUNCHBOX_7Z = "${pkgs.p7zip}/bin/7z";
          cargoBuildFlags = [ "--package" "lunchbox-db" ];
          cargoTestFlags = [ "--package" "lunchbox-db" ];
        };
        frontend = pkgs.rustPlatform.buildRustPackage {
          pname = "lunchbox";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [
            cmake
            ninja
            p7zip
            pkg-config
            qt6.wrapQtAppsHook
          ];
          buildInputs = qtModules;
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
          '';
          preFixup = ''
            qtWrapperArgs+=(--set LUNCHBOX_DATABASE "$out/share/lunchbox/lunchbox.db")
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
        packages = {
          default = frontend;
          lunchbox = frontend;
          lunchbox-db = databaseTool;
        };

        apps.default = {
          type = "app";
          program = "${frontend}/bin/lunchbox";
        };

        apps.lunchbox-db = {
          type = "app";
          program = "${databaseTool}/bin/lunchbox-db";
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
          ]) ++ qtModules;

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
