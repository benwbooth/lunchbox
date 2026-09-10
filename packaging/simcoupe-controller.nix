{ lib, stdenv, fetchurl, zlib, jq }:

stdenv.mkDerivation {
  pname = "lunchbox-simcp-controller";
  version = "0-unstable-c280462-controller1";
  src = fetchurl {
    url = "https://codeload.github.com/libretro/libretro-simcoupe/tar.gz/c28046241ac6a4d79e55326b6e354dc02f92fa34";
    hash = "sha256-PGHyAKq9udI+2YkGUJkOhiTtt8zoOnUCIy7XDt5r73o=";
    name = "libretro-simcoupe-c280462.tar.gz";
  };
  patches = [ ./simcoupe-controller-input.patch ];
  buildInputs = [ zlib ];
  nativeBuildInputs = [ jq ];
  makeFlags = [ "platform=unix" ];
  enableParallelBuilding = true;
  dontConfigure = true;
  installPhase = ''
    runHook preInstall
    install -Dm755 libretro-simcp.so "$out/lib/libretro/simcp_libretro.so"
    install -Dm644 SimCoupe/License.txt "$out/share/licenses/lunchbox-simcp-controller/License.txt"
    coreDigest=$(sha256sum "$out/lib/libretro/simcp_libretro.so")
    coreDigest=''${coreDigest%% *}
    jq -n --arg sha256 "$coreDigest" '{
      schema_version: 1,
      contract: "lunchbox-simcp-controller1",
      source_revision: "c28046241ac6a4d79e55326b6e354dc02f92fa34",
      library_name: "SimCoupe",
      library_version: "v1-lunchbox-controller1",
      core_sha256: $sha256
    }' > "$out/lib/libretro/simcp_libretro.so.controller.json"
    runHook postInstall
  '';
  meta = {
    description = "Opt-in SimCoupe libretro core with joystick fire and mode-release wiring";
    homepage = "https://github.com/libretro/libretro-simcoupe";
    license = lib.licenses.gpl2Plus;
    platforms = [ "x86_64-linux" "aarch64-linux" ];
  };
}
